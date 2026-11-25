//! Textos mostrados pelo lado Rust do app (o resto fica em `ui/src/texto.ts`).

pub const WEBVIEW2_MISSING: &str = "O Save Control precisa do WebView2 da Microsoft, que não foi encontrado neste PC.\n\nBaixe e instale em:\nhttps://developer.microsoft.com/microsoft-edge/webview2/";

pub const DATA_FALLBACK: &str =
    "A pasta do Save Control não permite gravar arquivos. Os dados ficam em %LOCALAPPDATA%\\SaveControl.";

pub const CONFIG_RESET: &str = "As configurações estavam danificadas e voltaram ao padrão.";

pub const CONFIG_UNREADABLE: &str =
    "Não foi possível ler as configurações agora. O app abriu com os padrões e não vai gravar por cima delas.";
