/*!
在arceos/exercises/support_hashmap/src/main.rs中，有：
```
#[macro_use]
#[cfg(feature = "axstd")]
extern crate axstd as std;
```
而在arceos/Cargo.toml中，有说明axstd的路径：axstd = { path = "ulib/axstd" }，
所以，arceos/exercises/support_hashmap/src/main.rs就把axstd当成了std来用（伪装支持std）。

现在要让arceos/exercises/support_hashmap/src/main.rs能use std::collections::HashMap，
在这里建一个collections包，在arceos/ulib/axstd/src/lib.rs中，把pub use alloc::collections去掉，
改成pub mod collections，这样就让lib.rs中定义的collections模块变成我写的collections模块。
然后其它类如BTreeMap仍然用alloc::collections中的。
*/

#[cfg(feature = "alloc")]
#[doc(no_inline)]
pub use alloc::collections::*; // 把 alloc::collections::* 引入到我的 collections 包下，这样就可以 collections::BTreeMap 这样访问

pub mod hashmap; // 声明有个模块叫hashmap，对应hashmap.rs
pub use hashmap::HashMap; // 这样，我的collections模块就有个HashMap，可以用collections::HashMap访问到