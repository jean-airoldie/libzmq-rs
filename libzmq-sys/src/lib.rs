// Ignore generated code
#![allow(clippy::all)]
#![doc(html_root_url = "https://docs.rs/libzmq-sys/0.1.0")]

//! libzmq-sys - Raw cFFI bindings to [libzmq](https://github.com/zeromq/libzmq).

#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]

include!(concat!(env!("OUT_DIR"), "/bindings.rs"));

pub mod errno;

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
