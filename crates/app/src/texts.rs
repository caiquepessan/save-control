//! Textos mostrados pelo lado Rust do app (o resto fica em `ui/src/texto.ts`).

pub const WEBVIEW2_MISSING: &str = "O Save Control precisa do WebView2 da Microsoft, que não foi encontrado neste PC.\n\nBaixe e instale em:\nhttps://developer.microsoft.com/microsoft-edge/webview2/";

pub const DATA_FALLBACK: &str =
    "A pasta do Save Control não permite gravar arquivos. Os dados ficam em %LOCALAPPDATA%\\SaveControl.";

pub const CONFIG_RESET: &str = "As configurações estavam danificadas e voltaram ao padrão.";

pub const BACKUP_APAGADO: &str = "Backup apagado";

/// "Iniciar com o Windows" apontava para um `.exe` num HD removível.
pub fn inicio_removivel(antigo: &str) -> String {
    format!("O Save Control iniciava junto com o Windows a partir de {antigo}, num HD removível. Agora ele inicia desta cópia.")
}

pub const COFRE_VOLTOU: &str = "O cofre voltou. O backup automático continua de onde parou.";

pub const TEMPO_SEGUNDOS: &str = "Use um valor entre 5 e 600 segundos.";

pub const TEMPO_MINUTOS: &str = "Use um valor entre 5 e 120 minutos.";

/// Tamanho como a interface mostra: KB abaixo de 512 KB, MB abaixo de 512 MB, senão GB.
pub fn tamanho(bytes: u64) -> String {
    const KB: f64 = 1024.0;
    let b = bytes as f64;
    if b < 512.0 * KB {
        format!("{} KB", (b / KB).round() as u64)
    } else if b < 512.0 * KB * KB {
        format!("{:.1} MB", b / (KB * KB)).replace('.', ",")
    } else {
        format!("{:.1} GB", b / (KB * KB * KB)).replace('.', ",")
    }
}

pub fn arquivos(n: usize, bytes: u64) -> String {
    format!("{n} {} · {}", if n == 1 { "arquivo" } else { "arquivos" }, tamanho(bytes))
}

pub fn restaurados(n: usize, seguranca: bool) -> String {
    let base = if n == 1 { "1 arquivo restaurado".to_string() } else { format!("{n} arquivos restaurados") };
    if seguranca { format!("{base} · o save anterior ficou no histórico") } else { base }
}

pub const CONFIG_UNREADABLE: &str =
    "Não foi possível ler as configurações agora. O app abriu com os padrões e não vai gravar por cima delas.";
