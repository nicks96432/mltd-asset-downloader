use num_derive::{FromPrimitive, ToPrimitive};

/// From [UnityPy](
///     https://github.com/K0lb3/UnityPy/blob/master/UnityPy/enums/BuildTarget.py
/// )
#[derive(Debug, Clone, Copy, PartialEq, Eq, FromPrimitive, ToPrimitive)]
pub enum Platform {
    DashboardWidget = 1,
    StandaloneOSX,
    StandaloneOSXPPC,
    StandaloneOSXIntel,
    StandaloneWindows,
    WebPlayer,
    WebPlayerStreamed,
    Wii,
    Ios,
    PS3,
    XBOX360,
    Android,
    StandaloneGLESEmu,
    NaCl,
    StandaloneLinux,
    FlashPlayer,
    StandaloneWindows64,
    WebGL,
    WSAPlayer,
    StandaloneLinux64,
    StandaloneLinuxUniversal,
    WP8Player,
    StandaloneOSXIntel64,
    BlackBerry,
    Tizen,
    PSP2,
    PS4,
    PSM,
    XboxOne,
    SamsungTV,
    N3DS,
    WiiU,
    TVOS,
    Switch,

    UnknownPlatform = 3716,
    NoTarget = -2,
}
