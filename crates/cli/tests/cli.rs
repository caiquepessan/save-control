use std::process::Command;

#[test]
fn ajuda_lista_os_comandos() {
    let out = Command::new(env!("CARGO_BIN_EXE_savectl")).arg("--help").output().unwrap();
    assert!(out.status.success());
    let texto = String::from_utf8_lossy(&out.stdout);
    for comando in ["escanear", "backup", "historico", "restaurar", "adicionar", "atualizar-base", "pastas-de-jogos"] {
        assert!(texto.contains(comando), "faltou {comando} em:\n{texto}");
    }
}
