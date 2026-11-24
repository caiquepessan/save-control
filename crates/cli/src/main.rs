//! `savectl`: linha de comando para testar o núcleo do Save Control.

use std::collections::BTreeMap;
use std::path::PathBuf;

use anyhow::{Context, bail};
use clap::{Parser, Subcommand};
use savecontrol_core::{
    backup::BackupOutcome,
    engine::{Engine, EngineConfig},
    env::SystemPaths,
    manifest::store::{HttpFetcher, UpdateOutcome},
    process::SystemProbe,
    registry::WinRegistry,
    restore::RestoreOptions,
    status::GameStatus,
    vault::format::SnapshotKind,
};

#[derive(Parser)]
#[command(name = "savectl", version, about = "Save Control: encontra, guarda e restaura saves de jogos")]
struct Cli {
    /// Pasta dos dados do app (base de jogos, índice, jogos à mão)
    #[arg(long, global = true, default_value = "SaveControl-dados")]
    dados: PathBuf,
    /// Pasta do cofre de backups
    #[arg(long, global = true, default_value = "cofre")]
    cofre: PathBuf,
    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Subcommand)]
enum Cmd {
    /// Define as pastas onde ficam seus jogos (ex.: D:\Jogos)
    PastasDeJogos { pastas: Vec<PathBuf> },
    /// Baixa ou atualiza a base de jogos de PC
    AtualizarBase,
    /// Lista os jogos encontrados e o status de cada um
    Escanear {
        #[arg(long)]
        json: bool,
    },
    /// Faz um backup manual de um jogo
    Backup {
        id: String,
        #[arg(long)]
        nome: Option<String>,
    },
    /// Lista os backups de um jogo
    Historico { id: String },
    /// Restaura um backup (--destino ALIAS=PASTA restaura em outra pasta)
    Restaurar {
        id: String,
        arquivo: String,
        #[arg(long = "destino")]
        destinos: Vec<String>,
    },
    /// Adiciona um jogo à mão
    Adicionar {
        #[arg(long)]
        nome: String,
        #[arg(long = "pasta", required = true)]
        pastas: Vec<PathBuf>,
        #[arg(long = "processo")]
        processos: Vec<String>,
    },
}

fn status_text(s: GameStatus) -> &'static str {
    match s {
        GameStatus::Protected => "Protegido",
        GameStatus::NewSave => "Save novo",
        GameStatus::NoBackup => "Sem backup",
        GameStatus::NoSave => "Sem save",
    }
}

fn kind_text(k: SnapshotKind) -> &'static str {
    match k {
        SnapshotKind::Manual => "manual",
        SnapshotKind::Auto => "automático",
        SnapshotKind::PreRestore => "antes de restaurar",
        SnapshotKind::Pinned => "fixado",
    }
}

fn size_text(bytes: u64) -> String {
    const UNITS: [&str; 4] = ["B", "KB", "MB", "GB"];
    let mut v = bytes as f64;
    let mut u = 0;
    while v >= 1024.0 && u < UNITS.len() - 1 {
        v /= 1024.0;
        u += 1;
    }
    if u == 0 { format!("{bytes} B") } else { format!("{v:.1} {}", UNITS[u]).replace('.', ",") }
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    let sys = SystemPaths::detect()?;
    let cfg = EngineConfig { data_dir: cli.dados, vault_dir: cli.cofre, include_config: false };
    let mut eng = Engine::open(cfg, sys, Box::new(WinRegistry)).context("não foi possível abrir o cofre")?;

    match cli.cmd {
        Cmd::PastasDeJogos { pastas } => {
            eng.set_user_roots(pastas)?;
            println!("Pastas de jogos salvas.");
        }
        Cmd::AtualizarBase => match eng.update_manifest(&HttpFetcher)? {
            UpdateOutcome::Updated => println!("Base de jogos baixada."),
            UpdateOutcome::NotModified => println!("A base de jogos já está atualizada."),
            UpdateOutcome::Skipped => println!("A base foi conferida nas últimas 24 h."),
        },
        Cmd::Escanear { json } => {
            let jogos = eng.scan()?;
            if json {
                let lista: Vec<_> = jogos.iter().map(|(g, s)| serde_json::json!({ "jogo": g, "status": s })).collect();
                println!("{}", serde_json::to_string_pretty(&lista)?);
            } else {
                println!("{:<11} {:>5} {:>10}  {:<17}  {}", "STATUS", "ARQ.", "TAMANHO", "ÚLTIMO SAVE", "ID / NOME");
                for (g, s) in &jogos {
                    let quando = g
                        .last_saved
                        .map(|d| d.with_timezone(&chrono::Local).format("%d/%m/%Y %H:%M").to_string())
                        .unwrap_or_else(|| "—".into());
                    println!(
                        "{:<11} {:>5} {:>10}  {:<17}  {}  ({})",
                        status_text(*s),
                        g.files.len(),
                        size_text(g.total_size),
                        quando,
                        g.id,
                        g.title
                    );
                }
                println!("\n{} jogos.", jogos.len());
            }
        }
        Cmd::Backup { id, nome } => {
            eng.scan()?;
            match eng.backup(&id, SnapshotKind::Manual, nome.as_deref())? {
                BackupOutcome::Created(e) => println!("Backup criado: {}", e.file_name),
                BackupOutcome::Unchanged(e) => println!("Já existe um backup igual: {}", e.file_name),
                BackupOutcome::NothingToBackup => println!("Esse jogo não tem arquivos de save agora."),
            }
        }
        Cmd::Historico { id } => {
            let h = eng.history(&id);
            if h.is_empty() {
                println!("Nenhum backup de {id}.");
            }
            for e in h {
                println!(
                    "{}  {:<18}  {:>3} arquivos  {:>10}  {}",
                    e.file_name,
                    kind_text(e.kind),
                    e.file_count,
                    size_text(e.total_size),
                    e.label.unwrap_or_default()
                );
            }
        }
        Cmd::Restaurar { id, arquivo, destinos } => {
            let mut overrides = BTreeMap::new();
            for d in destinos {
                let Some((alias, pasta)) = d.split_once('=') else { bail!("use --destino ALIAS=PASTA (ex.: r0=C:\\temp)") };
                overrides.insert(alias.to_string(), PathBuf::from(pasta));
            }
            eng.scan()?;
            let opts = RestoreOptions { overrides, only: None };
            let r = eng.restore(&id, &arquivo, &opts, &SystemProbe)?;
            println!("Restaurado: {} arquivo(s) gravado(s), {} removido(s).", r.written, r.removed);
            if let Some(s) = r.safety {
                println!("O save de antes está guardado em: {}", s.file_name);
            }
        }
        Cmd::Adicionar { nome, pastas, processos } => {
            let g = eng.add_custom_game(&nome, &pastas, &processos)?;
            println!("Jogo adicionado: {} ({})", g.name, g.id);
        }
    }
    Ok(())
}
