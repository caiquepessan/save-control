# Save Control

> Cofre de saves portátil para Windows: encontra os saves dos seus jogos, faz backup automático e restaura com um clique.

**Status:** em design. Ainda não tem código.

## O problema

Cada jogo guarda o save num lugar diferente: AppData, Documentos, a pasta do próprio jogo, a pasta do emulador. Na hora de atualizar um jogo, trocar de PC ou formatar, é fácil perder o progresso sem nem saber onde ele estava.

## A ideia

O Save Control junta tudo num lugar só.

- **Encontra os saves sozinho**
  - Jogos de PC (Steam, Epic, GOG, EA, Ubisoft e outros), usando a base de locais de save do [PCGamingWiki](https://www.pcgamingwiki.com/), compilada pelo [ludusavi-manifest](https://github.com/mtkennerly/ludusavi-manifest).
  - Ports e recompilações para PC (como os projetos baseados no N64Recomp, inclusive no modo portátil) e emuladores (Project64, RetroArch e outros), por meio de um catálogo próprio.
  - Qualquer outro jogo, adicionado à mão, com um modo "detetive" que mostra quais arquivos mudaram logo depois que você salvou no jogo.
- **Cofre portátil:** uma única pasta, ao lado do app ou num HD externo, com o histórico de backups de cada jogo.
- **Backup com nome** ("antes de atualizar") quando você quiser, e **backup automático** em segundo plano sempre que um jogo salva.
- **Restauração segura:** antes de restaurar, o save atual vira um backup. Nada se perde.
- **Leva pra outro PC:** os caminhos ficam guardados de forma genérica (AppData do usuário, pasta da Steam...), então a restauração funciona mesmo com outro usuário ou outro disco.
- **Sem instalação:** roda direto de uma pasta ou de um pendrive.

## Créditos

Os locais de save dos jogos de PC vêm do PCGamingWiki, via ludusavi-manifest. Projeto inspirado no [Ludusavi](https://github.com/mtkennerly/ludusavi).
