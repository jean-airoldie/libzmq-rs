// Ignore generated code
#![allow(clippy::all)]
#![doc(html_root_url = "https://docs.rs/libzmq-sys/0.1.7")]

//! libzmq-sys - Raw cFFI bindings to [libzmq](https://github.com/zeromq/libzmq).

#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]

mod bindings;
pub mod errno;
#[cfg(windows)]
pub(crate) mod windows;

pub use bindings::*;

#[cfg(test)]
mod test {
    #[test]
    fn test_readme_deps() {
        version_sync::assert_markdown_deps_updated!("README.md");
    }

    #[test]
    fn test_html_root_url() {
        version_sync::assert_html_root_url_updated!("src/lib.rs");
    }
}
