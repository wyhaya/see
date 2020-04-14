pub const SIGTERM: i32 = 15;

extern "C" {
    #[cfg_attr(
        all(target_os = "macos", target_arch = "x86"),
        link_name = "kill$UNIX2003"
    )]
    pub fn kill(pid: i32, sig: i32) -> i32;
}
