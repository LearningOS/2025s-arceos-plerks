use alloc::collections::BTreeMap;
use alloc::sync::{Arc, Weak};
use alloc::{string::String, vec::Vec};

use axfs_vfs::{VfsDirEntry, VfsNodeAttr, VfsNodeOps, VfsNodeRef, VfsNodeType};
use axfs_vfs::{VfsError, VfsResult};
use spin::RwLock;

use crate::file::FileNode;

/// The directory node in the RAM filesystem.
///
/// It implements [`axfs_vfs::VfsNodeOps`].
pub struct DirNode {
    this: Weak<DirNode>,
    parent: RwLock<Weak<dyn VfsNodeOps>>,
    children: RwLock<BTreeMap<String, VfsNodeRef>>,
}

impl DirNode {
    pub(super) fn new(parent: Option<Weak<dyn VfsNodeOps>>) -> Arc<Self> {
        Arc::new_cyclic(|this| Self {
            this: this.clone(),
            parent: RwLock::new(parent.unwrap_or_else(|| Weak::<Self>::new())),
            children: RwLock::new(BTreeMap::new()),
        })
    }

    pub(super) fn set_parent(&self, parent: Option<&VfsNodeRef>) {
        *self.parent.write() = parent.map_or(Weak::<Self>::new() as _, Arc::downgrade);
    }

    /// Returns a string list of all entries in this directory.
    pub fn get_entries(&self) -> Vec<String> {
        self.children.read().keys().cloned().collect()
    }

    /// Checks whether a node with the given name exists in this directory.
    pub fn exist(&self, name: &str) -> bool {
        self.children.read().contains_key(name)
    }

    /// Creates a new node with the given name and type in this directory.
    pub fn create_node(&self, name: &str, ty: VfsNodeType) -> VfsResult {
        if self.exist(name) {
            log::error!("AlreadyExists {}", name);
            return Err(VfsError::AlreadyExists);
        }
        let node: VfsNodeRef = match ty {
            VfsNodeType::File => Arc::new(FileNode::new()),
            VfsNodeType::Dir => Self::new(Some(self.this.clone())),
            _ => return Err(VfsError::Unsupported),
        };
        self.children.write().insert(name.into(), node);
        Ok(())
    }

    /// Removes a node by the given name in this directory.
    pub fn remove_node(&self, name: &str) -> VfsResult {
        let mut children = self.children.write();
        let node = children.get(name).ok_or(VfsError::NotFound)?;
        if let Some(dir) = node.as_any().downcast_ref::<DirNode>() {
            if !dir.children.read().is_empty() {
                return Err(VfsError::DirectoryNotEmpty);
            }
        }
        children.remove(name);
        Ok(())
    }
}

impl VfsNodeOps for DirNode {
    fn get_attr(&self) -> VfsResult<VfsNodeAttr> {
        Ok(VfsNodeAttr::new_dir(4096, 0))
    }

    fn parent(&self) -> Option<VfsNodeRef> {
        self.parent.read().upgrade()
    }

    fn lookup(self: Arc<Self>, path: &str) -> VfsResult<VfsNodeRef> {
        let (name, rest) = split_path(path);
        let node = match name {
            "" | "." => Ok(self.clone() as VfsNodeRef),
            ".." => self.parent().ok_or(VfsError::NotFound),
            _ => self
                .children
                .read()
                .get(name)
                .cloned()
                .ok_or(VfsError::NotFound),
        }?;

        if let Some(rest) = rest {
            node.lookup(rest)
        } else {
            Ok(node)
        }
    }

    fn read_dir(&self, start_idx: usize, dirents: &mut [VfsDirEntry]) -> VfsResult<usize> {
        let children = self.children.read();
        let mut children = children.iter().skip(start_idx.max(2) - 2);
        for (i, ent) in dirents.iter_mut().enumerate() {
            match i + start_idx {
                0 => *ent = VfsDirEntry::new(".", VfsNodeType::Dir),
                1 => *ent = VfsDirEntry::new("..", VfsNodeType::Dir),
                _ => {
                    if let Some((name, node)) = children.next() {
                        *ent = VfsDirEntry::new(name, node.get_attr().unwrap().file_type());
                    } else {
                        return Ok(i);
                    }
                }
            }
        }
        Ok(dirents.len())
    }

    fn create(&self, path: &str, ty: VfsNodeType) -> VfsResult {
        log::debug!("create {:?} at ramfs: {}", ty, path);
        let (name, rest) = split_path(path);
        if let Some(rest) = rest {
            match name {
                "" | "." => self.create(rest, ty),
                ".." => self.parent().ok_or(VfsError::NotFound)?.create(rest, ty),
                _ => {
                    let subdir = self
                        .children
                        .read()
                        .get(name)
                        .ok_or(VfsError::NotFound)?
                        .clone();
                    subdir.create(rest, ty)
                }
            }
        } else if name.is_empty() || name == "." || name == ".." {
            Ok(()) // already exists
        } else {
            self.create_node(name, ty)
        }
    }

    fn remove(&self, path: &str) -> VfsResult {
        log::debug!("remove at ramfs: {}", path);
        let (name, rest) = split_path(path);
        if let Some(rest) = rest {
            match name {
                "" | "." => self.remove(rest),
                ".." => self.parent().ok_or(VfsError::NotFound)?.remove(rest),
                _ => {
                    let subdir = self
                        .children
                        .read()
                        .get(name)
                        .ok_or(VfsError::NotFound)?
                        .clone();
                    subdir.remove(rest)
                }
            }
        } else if name.is_empty() || name == "." || name == ".." {
            Err(VfsError::InvalidInput) // remove '.' or '..
        } else {
            self.remove_node(name)
        }
    }

    /// 把一个文件改名，
    /// 这参数设计得有点抽象，既然是把一个文件改名，那参数应该是file_path和new_name，但是实际是两个path，搞得好像移动一样，
    /// 而且传进来的_src_path是"/f1"，是绝对路径，但是这里的含义应该是想表达相对路径"f1"
    fn rename(&self, _src_path: &str, _dst_path: &str) -> VfsResult {
        /* _src_path是"/f1"，_dst_path是"/tmp/f2"。但是self.children的key是"f1"这样
        的格式，所以把_src_path和_dst_path处理一下，取最后一个'/'后面的子串
        */
        log::debug!("_src_path: {} _dst_path: {}", _src_path, _dst_path);
        let src_path = _src_path.rsplit('/').next().unwrap_or("");
        let dst_path = _dst_path.rsplit('/').next().unwrap_or("");
        log::debug!("src_path: {} dst_path: {}", src_path, dst_path);
        
        /* DirNode有个Weak<DirNode>属性引用自己，用来实现：变回Arc<DirNode>，这样就可以调用lookup()了。
        不能现造一个Arc，比如写let result = Arc::new(*self).lookup(_src_path)的话，会把*self move掉，
        这里传进来的是self借用，不允许move掉。而且Arc::new创建的新Arc，Arc内部存的(引用的)内容物是拷贝的，和原来那个Arc不一样。
        */
        let result = (*self).this.upgrade().unwrap().lookup(_src_path);
        if result.is_err() { // 检查_src_path对应文件的存在性
            return Err(VfsError::NotFound);
        }
        
        for name in self.children.read().keys() {
            log::debug!("name: {}", name);
        }
        
        let mut children = self.children.write();
        let node = children.remove(src_path).unwrap();
        children.insert(String::from(dst_path), node);
        drop(children); // drop掉children这把写锁，防止后面还要读self.children的话卡住

        for name in self.children.read().keys() {
            log::debug!("name: {}", name);
        }

        Ok(())
    }

    axfs_vfs::impl_vfs_dir_default! {}
}

fn split_path(path: &str) -> (&str, Option<&str>) {
    let trimmed_path = path.trim_start_matches('/');
    trimmed_path.find('/').map_or((trimmed_path, None), |n| {
        (&trimmed_path[..n], Some(&trimmed_path[n + 1..]))
    })
}
