mod common;

use std::path::{Path, PathBuf};

use common::{local, write};
use savecontrol_core::{
    error::Error,
    machine::MachineIdentity,
    model::GameSource,
    vault::{Vault, format::*, read::*, write::*},
};

fn pc() -> MachineIdentity {
    MachineIdentity {
        id: "3f9a1b2c3d4e5f60".into(),
        pc_name: "DESKTOP Ção 123 nome longo".into(),
        user_name: "teste".into(),
    }
}

fn meta() -> SnapshotMeta {
    SnapshotMeta {
        format: FORMAT,
        game_id: "pc:Jogo A".into(),
        title: "Jogo A".into(),
        kind: SnapshotKind::Manual,
        label: Some("antes de atualizar".into()),
        created_at: chrono::Utc::now(),
        app_version: "0.1.0".into(),
        machine: pc(),
        roots: vec![SnapshotRoot {
            alias: "r0".into(),
            template: "<winAppData>/JogoA".into(),
            bindings: Default::default(),
            original_path: "C:/x".into(),
        }],
        files: vec![],
    }
}

fn sources(dir: &Path) -> Vec<SourceFile> {
    vec![
        SourceFile { abs_path: dir.join("slot1.sav"), root_alias: "r0".into(), rel_path: "slot1.sav".into() },
        SourceFile { abs_path: dir.join("sub/slot2.sav"), root_alias: "r0".into(), rel_path: "sub/slot2.sav".into() },
    ]
}

fn sobras(dir: &Path) -> Vec<String> {
    let mut v: Vec<String> = std::fs::read_dir(dir)
        .unwrap()
        .map(|e| e.unwrap().file_name().into_string().unwrap())
        .filter(|n| n != "jogo.json")
        .collect();
    v.sort();
    v
}

fn origem(t: &Path) -> PathBuf {
    let src = t.join("src");
    write(&src.join("slot1.sav"), b"um");
    write(&src.join("sub/slot2.sav"), b"dois");
    src
}

#[test]
fn rotulo_do_pc() {
    assert_eq!(pc().label(), "DESKTOPo123nome-3F9A");
}

#[test]
fn nome_e_tipo_do_arquivo() {
    assert_eq!(
        snapshot_file_name(local(2026, 11, 24, 10, 2, 11), &pc(), SnapshotKind::Auto),
        "2026-11-24_10-02-11_DESKTOPo123nome-3F9A_auto.zip"
    );
    assert_eq!(
        SnapshotKind::from_file_name("2026-11-24_10-02-11_PC-ABCD_antes-de-restaurar.zip"),
        Some(SnapshotKind::PreRestore)
    );
    assert_eq!(SnapshotKind::from_file_name("2026-11-24_10-02-11-2_PC-ABCD_fixado.zip"), Some(SnapshotKind::Pinned));
    assert_eq!(SnapshotKind::from_file_name("2026-11-24_10-02-11_PC-ABCD_auto (1).zip"), None);
}

#[test]
fn limpa_nome_da_pasta() {
    assert_eq!(sanitize_folder_name("The Legend of Zelda: Ocarina of Time"), "The Legend of Zelda - Ocarina of Time");
    assert_eq!(sanitize_folder_name("a<b>c?"), "a_b_c_");
    assert_eq!(sanitize_folder_name("   "), "jogo");
}

#[test]
fn escreve_confere_e_le_o_zip() {
    let t = tempfile::tempdir().unwrap();
    let src = origem(t.path());
    let v = Vault::open_or_create(&t.path().join("cofre")).unwrap();
    let dir = v.game_dir("pc:Jogo A", "Jogo A").unwrap();
    let name = snapshot_file_name(local(2026, 11, 24, 10, 2, 11), &pc(), SnapshotKind::Manual);
    let (path, m) = write_snapshot(&dir, &name, meta(), &sources(&src), &NoWait).unwrap();
    assert_eq!(m.files.len(), 2);
    assert_eq!(m.files[0].blake3, blake3::hash(b"um").to_hex().to_string());
    assert_eq!(read_meta(&path).unwrap(), m);
    assert_eq!(sobras(&dir), [name.clone()]);
    let (path2, _) = write_snapshot(&dir, &name, meta(), &sources(&src), &NoWait).unwrap();
    assert!(path2.file_name().unwrap().to_str().unwrap().starts_with("2026-11-24_10-02-11-2_"));
    let out = t.path().join("out.sav");
    extract_file(&path, "r0", "sub/slot2.sav", &out).unwrap();
    assert_eq!(std::fs::read(out).unwrap(), b"dois");
}

#[test]
fn pastas_de_jogos_diferentes_com_mesmo_nome() {
    let t = tempfile::tempdir().unwrap();
    let v = Vault::open_or_create(&t.path().join("cofre")).unwrap();
    let a = v.game_dir("pc:Jogo", "Jogo").unwrap();
    v.write_descriptor(
        &a,
        &GameDescriptor {
            format: FORMAT,
            game_id: "pc:Jogo".into(),
            title: "Jogo".into(),
            source: GameSource::ManifestOnly,
            custom: None,
            steam_app_id: None,
            process_names: vec![],
        },
    )
    .unwrap();
    let b = v.game_dir("man:9", "Jogo").unwrap();
    assert_ne!(a, b);
    assert!(b.file_name().unwrap().to_str().unwrap().starts_with("Jogo ("));
    assert_eq!(v.game_dir("pc:Jogo", "Jogo").unwrap(), a);
}

struct Muda(PathBuf);

impl CopyHooks for Muda {
    fn after_read(&self, p: &Path) {
        if p == self.0 {
            let mut b = std::fs::read(p).unwrap();
            b.push(b'!');
            std::fs::write(p, b).unwrap();
        }
    }
}

#[test]
fn arquivo_instavel_desiste_limpo() {
    let t = tempfile::tempdir().unwrap();
    let src = origem(t.path());
    let v = Vault::open_or_create(&t.path().join("cofre")).unwrap();
    let dir = v.game_dir("pc:Jogo A", "Jogo A").unwrap();
    let err = write_snapshot(&dir, "x_PC-AAAA_manual.zip", meta(), &sources(&src), &Muda(src.join("slot1.sav")))
        .unwrap_err();
    assert!(matches!(err, Error::UnstableFile(_)));
    assert!(sobras(&dir).is_empty());
}

#[cfg(windows)]
#[test]
fn arquivo_travado_desiste_limpo() {
    use std::os::windows::fs::OpenOptionsExt;
    let t = tempfile::tempdir().unwrap();
    let src = origem(t.path());
    let _trava = std::fs::OpenOptions::new().read(true).share_mode(0).open(src.join("slot1.sav")).unwrap();
    let v = Vault::open_or_create(&t.path().join("cofre")).unwrap();
    let dir = v.game_dir("pc:Jogo A", "Jogo A").unwrap();
    let err = write_snapshot(&dir, "x_PC-AAAA_manual.zip", meta(), &sources(&src), &NoWait).unwrap_err();
    assert!(matches!(err, Error::FileLocked(_)));
    assert!(sobras(&dir).is_empty());
}

#[test]
fn limpa_so_parciais_deste_pc_ou_velhos() {
    let t = tempfile::tempdir().unwrap();
    let v = Vault::open_or_create(&t.path().join("cofre")).unwrap();
    let dir = v.game_dir("pc:Jogo A", "Jogo A").unwrap();
    let meu = dir.join(format!("2026-11-24_10-00-00_{}_auto.zip.parcial", pc().label()));
    let outro = dir.join("2026-11-24_10-00-00_OUTRO-1111_auto.zip.parcial");
    let velho = dir.join("2026-11-20_10-00-00_OUTRO-1111_auto.zip.parcial");
    for f in [&meu, &outro, &velho] {
        std::fs::write(f, b"x").unwrap();
    }
    let ha_dois_dias = std::time::SystemTime::now() - std::time::Duration::from_secs(48 * 3600);
    let arquivo = std::fs::File::options().write(true).open(&velho).unwrap();
    arquivo.set_times(std::fs::FileTimes::new().set_modified(ha_dois_dias)).unwrap();
    drop(arquivo);
    assert_eq!(v.cleanup_partials(&pc(), chrono::Utc::now()).unwrap(), 2);
    assert!(outro.exists() && !meu.exists() && !velho.exists());
}
