// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    // NVIDIA proprietary driver + Wayland (Hyprland 等) では、WebKitGTK が explicit sync で
    // 失敗し、コンポジタが Gdk Error 71 (プロトコルエラー) でクライアントを切断する。
    // NVIDIA ドライバだけが読む変数なので AMD / Intel では無視される。
    // WebKitGTK / NVIDIA 側で解消したら外してよい。
    // 効かない場合の次の手: WEBKIT_DISABLE_DMABUF_RENDERER=1 (高速描画経路を失う)。
    // - https://v2.tauri.app/develop/debug/linux-graphics/
    // - https://bugs.webkit.org/show_bug.cgi?id=280210
    //
    // スレッド起動前 (app_lib::run() より前) に設定する。ユーザー設定値は上書きしない。
    // edition 2021 のため unsafe 不要。edition 2024 に上げる際は unsafe + SAFETY コメントが必要。
    #[cfg(target_os = "linux")]
    if std::env::var_os("__NV_DISABLE_EXPLICIT_SYNC").is_none() {
        std::env::set_var("__NV_DISABLE_EXPLICIT_SYNC", "1");
    }

    app_lib::run();
}
