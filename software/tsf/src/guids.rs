// crates/tsf-service/src/guids.rs
use windows::core::GUID;

// 用 `uuidgen` 或 Rust 的 uuid crate 生成,这里只是占位示例
pub const CLSID_TEXT_SERVICE: GUID =
    GUID::from_u128(0x1c608d58_33c3_4e27_8cc5_e0d733878336);

pub const GUID_PROFILE: GUID =
    GUID::from_u128(0xd1213334_9fd9_475d_abf2_23cf60ffef20);

// 系统内置类别,不用自己生成
pub const GUID_LANGBAR_ITEM: GUID = // 可选,若要加语言栏图标
    GUID::from_u128(0xe8b3e9eb_b4c4_4616_86ec_b82bdb9a79f0);