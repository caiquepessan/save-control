# Save Control — Documento de design

**Data:** 24/11/2026 · **Status:** aguardando aprovação · **Repositório:** caiquepessan/save-control

---

## 1. Por que existe

Cada jogo guarda o save num lugar diferente: AppData, Documentos, Saved Games, a pasta do próprio jogo (ports e recompilações em modo portátil), a pasta do emulador. Quem joga ports, recompilações e emuladores troca de versão com frequência. Na hora de atualizar, trocar de PC ou limpar uma pasta, é fácil perder o progresso sem nem saber onde ele estava.

O Save Control é um app de Windows, portátil, que:

- **encontra sozinho** os saves dos jogos do PC;
- guarda cópias com histórico num **cofre**: uma pasta só, que dá pra levar num HD externo;
- faz backup **automático** em segundo plano e **manual com nome** ("antes de atualizar o recomp");
- **restaura** com segurança, por cima do save que o jogo criou.

O visual é parte do produto. Tem que parecer um launcher moderno, muito acima do Ludusavi, que serviu de inspiração e cuja base de dados aberta será reaproveitada.

**Sucesso, para o usuário:** atualizar um recomp sem medo; abrir o app e ver os jogos como "Protegido"; plugar o HD num PC novo e recuperar tudo.

---

## 2. Objetivos, fora do escopo e antiobjetivos

### Objetivos (v1)

1. Encontrar saves de:
   - jogos de PC, pela base do ludusavi-manifest (dados do PCGamingWiki);
   - recomps, ports e emuladores, por um catálogo próprio;
   - jogos adicionados à mão, com ajuda do modo **Detetive**.
2. Cofre portátil com histórico por jogo.
3. Backup manual (com nome opcional) e automático em segundo plano.
4. Restaurar qualquer versão, sempre guardando o save atual antes.
5. Restaurar em outro PC, com os caminhos traduzidos.
6. Interface da direção A com painel lateral (mockups aprovados).
7. Rodar sem instalação: um `.exe` e uma pasta.

### Fora do escopo (v1)

- Sincronização com nuvem. Dá pra pôr o cofre numa pasta do OneDrive ou Google Drive por conta própria.
- Linux e macOS.
- Saves guardados no registro do Windows. A base indica quando existem; a v1 só avisa.
- Saves da Microsoft Store / Xbox (contêineres WGS).
- Converter saves entre versões ou plataformas (ex.: save de emulador para recomp).
- Instalador, atualização automática do próprio app e assinatura de código.
- Outras línguas além de pt-BR. Os textos ficam centralizados para traduzir depois.

### Antiobjetivos (fracasso, mesmo que "funcione")

- Perder ou corromper um save: restaurar sem guardar o atual, gravar backup pela metade, ou apagar backups bons na limpeza automática.
- Ter cara de ferramenta genérica, o "visual do Ludusavi".
- Espalhar arquivos ou entradas de registro pelo sistema sem o usuário pedir. As únicas entradas possíveis são opcionais e removíveis (§13).
- Pesar durante o jogo: pico de CPU, disco martelando, notificação no meio da partida.
- Deixar rastro de IA no repositório: nada de "Co-Authored-By" de IA nos commits, nada de pastas de ferramentas versionadas.

---

## 3. Restrições

- **Windows 10/11, x64.** Depende do WebView2 Runtime (vem no Windows 11; no 10 quase sempre já está). Se faltar, mostrar uma mensagem com o link oficial da Microsoft.
- **Rust estável + Tauri 2;** interface em **React + TypeScript** (Vite).
- **Portátil:** tudo do app fica ao lado do `.exe` por padrão.
- **Repositório público:** nada de caminhos pessoais, nomes de usuário ou referências a cópias não oficiais de jogos, em código, testes, exemplos ou documentação.
- **Commits** só com a identidade git do usuário, sem atribuição a IA.
- **Interface em pt-BR.**

---

## 4. Abordagens consideradas

### Interface (em todas, a lógica é Rust)

| | Como funciona | A favor | Contra |
|---|---|---|---|
| **1. Tauri 2 + React (escolhida)** | Núcleo em Rust; tela em HTML/CSS/TypeScript, exibida pelo WebView2 | Reproduz os mockups fielmente (blur, gradientes, animações). Ajuste visual aparece na hora (hot reload). Bandeja, notificações, iniciar com o Windows e instância única já existem como plugins. `.exe` pequeno. | Duas linguagens no projeto. ~100–150 MB de RAM com a janela aberta; isso some quando o app vai pra bandeja. |
| 2. Dioxus | A tela também é escrita em Rust, renderizada no WebView2 | Uma linguagem só; mesmo visual | Ecossistema bem menor; cada ajuste visual exige recompilar o Rust |
| 3. Nativo (Slint ou Iced) | Sem WebView | Menos RAM | Não chega no visual escolhido (o Iced é o que deixa o Ludusavi com aquela cara) |

O usuário pediu Rust e delegou a escolha da interface ("o que você achar melhor"). A escolhida foi a **1**, porque é a única que garante que o app fica igual aos mockups aprovados.

### Motor de detecção

Motor próprio, em Rust, lendo a mesma base aberta que o Ludusavi usa, em vez de embutir o executável do Ludusavi. Embutir daria a detecção pronta, mas prenderia o cofre, os backups com nome e o backup automático ao formato e ao ritmo dele, além de levar um segundo `.exe` junto.

---

## 5. Arquitetura

```
save-control/
  Cargo.toml              workspace
  crates/core/            biblioteca Rust, sem Tauri (testável sozinha)
    paths/ manifest/ stores/ catalog/ scan/ vault/ backup/ restore/ watch/ detective/ process/
  src-tauri/              app Tauri: comandos, eventos, janela, bandeja, notificações
  ui/                     React + TypeScript + Vite
  catalog/                catálogo próprio em YAML, embutido no build
  docs/design/            este documento
```

- **`core`** não conhece a interface. Recebe as pastas do sistema e a configuração, e devolve dados. Toda regra de negócio mora aqui.
- **`src-tauri`** expõe comandos (escanear, listar jogos, backup, restaurar, histórico, ajustes, detetive…) e emite eventos (progresso do escaneamento, jogo atualizado, backup feito, situação do cofre…).
- Um **serviço interno** (async, no próprio processo) continua rodando quando a janela fecha: vigia as pastas, faz os backups automáticos e mostra as notificações. A janela (WebView) é **destruída** ao ir pra bandeja e recriada ao abrir, para liberar memória.
- **Instância única:** abrir o `.exe` de novo traz a janela que já existe.

---

## 6. Onde ficam os dados (modo portátil)

Ao lado do `.exe`:

```
SaveControl.exe
SaveControl-dados/
  config.json        ajustes
  jogos.json         ajustes deste PC por jogo (pastas confirmadas aqui, capa, ignorar)
  indice/            cache do índice do cofre (nunca fica dentro do cofre)
  base/              base de jogos de PC baixada + ETag + cache pré-processado
  capas/             capas baixadas ou escolhidas pelo usuário
  webview/           dados do WebView2 (para não sujar o AppData)
  atividade.jsonl    histórico de backups, restaurações e erros
  logs/
cofre/               cofre padrão (pode ser movido para qualquer pasta ou HD)
```

- Se a pasta do `.exe` não for gravável (ex.: dentro de Program Files), usar `%LOCALAPPDATA%\SaveControl` e avisar.
- O cofre é autossuficiente: outro Save Control, em outro PC, apontado para ele, enxerga tudo.
- A definição de cada jogo adicionado à mão fica no cofre (§8.1), para ele aparecer em outro PC. O que é específico deste PC (onde a pasta está aqui) fica no `jogos.json`.

---

## 7. Encontrar saves

### 7.1 Base de jogos de PC

- **Fonte:** `https://raw.githubusercontent.com/mtkennerly/ludusavi-manifest/refs/heads/data/data/manifest.yaml` (~17 MB).
- **Atualização:** baixar no primeiro uso. Depois, conferir no máximo uma vez por dia com `If-None-Match` usando o `ETag` guardado: 304 significa "sem novidade", 200 significa "guardar a nova versão e o novo ETag".
- **Sem internet no primeiro uso:** o app funciona com o catálogo próprio e os jogos à mão, e mostra o aviso "Baixe a base de jogos de PC quando estiver online".
- **Formato:** mapa `título → entrada`. Seguir o esquema "leniente" do projeto (aceitar valores desconhecidos sem quebrar). Campos usados:
  - `files`: caminho → `{ tags: [save|config], when: [{ os, store }] }`;
  - `registry` (só para avisar, ver §15);
  - `installDir`: nome da pasta → `{}`;
  - `steam.id`, `gog.id`, `id.steamExtra`, `id.gogExtra`;
  - `alias`: aponta outro título; o resto da entrada é ignorado;
  - `cloud` (ver §15) e `notes`.
- **Placeholders no Windows:** `<root>`, `<game>`, `<base>`, `<home>`, `<storeGameId>`, `<storeUserId>`, `<osUserName>`, `<winAppData>`, `<winLocalAppData>`, `<winLocalAppDataLow>`, `<winDocuments>`, `<winPublic>`, `<winProgramData>`, `<winDir>`.
  - Primeiro substitui os placeholders, depois avalia como glob.
  - Uma pasta inclui tudo dentro dela.
  - `[` escapa caracteres especiais (`[[]`).
- **Filtro:** só entradas sem `when` ou com `os: windows`. Por padrão, só arquivos com a tag `save`; `config` é opcional nos ajustes.
- **Desempenho:** fazer o parse de 17 MB de YAML demora. Guardar um cache binário pré-processado (chave = ETag) para o app abrir rápido.
- O manifest secundário `.ludusavi.yaml` dentro da pasta de um jogo pode ser suportado se for barato (fica a critério).

### 7.2 Lojas e pastas de jogos

Detectar automaticamente:

- **Steam:**
  - caminho pelo registro (`HKCU\Software\Valve\Steam`, valor `SteamPath`);
  - bibliotecas em `steamapps/libraryfolders.vdf`;
  - jogos instalados pelos `appmanifest_*.acf` (campo `installdir`);
  - `<base>` = `<root>/steamapps/common/<game>`;
  - `<storeUserId>` = as pastas dentro de `userdata/`.
  - O app ID vem de `steam.id` e também serve para buscar capas.
- **Epic:** arquivos JSON em `C:\ProgramData\Epic\EpicGamesLauncher\Data\Manifests\*.item` (`InstallLocation`, `DisplayName`).
- **GOG:** registro `HKLM\SOFTWARE\WOW6432Node\GOG.com\Games\*` (`path`, `gameName`).
- **Pastas de jogos do usuário** (ex.: `D:\Jogos`), configuráveis. Cada subpasta é tratada como um jogo, casando pelo `installDir` ou pelo título.
- **EA, Ubisoft, Battle.net e outras:** na v1, cobertas pelo que a base resolve sem `<root>` (AppData, Documentos…) e pelas pastas de jogos do usuário. Detecção dedicada fica para depois.

### 7.3 Catálogo próprio (recomps, ports e emuladores)

Arquivo YAML no repositório, embutido no build, no mesmo espírito da base. Cada entrada diz como achar o programa e onde ficam os saves. Campos sugeridos:

- `id`, `nome` e `tipo` (recomp, port ou emulador);
- `exe`: nomes possíveis do executável;
- `onde_procurar`: pastas típicas, mais as pastas de jogos do usuário;
- `saves`: padrões relativos ao `.exe` ou ao AppData;
- `regra_portatil`. Exemplo: se existe `portable.txt` ao lado do `.exe`, os saves ficam em `<pasta do exe>/saves`; senão, em `%LOCALAPPDATA%\<Nome>\saves`;
- `processos`: para saber se o programa está aberto;
- `separar_por_jogo`: como dividir os saves de um emulador em um item por jogo.

Cobertura inicial (o caminho exato de cada um deve ser conferido na implementação):

- **Família N64Recomp** (ex.: Zelda64Recompiled, BanjoRecompiled e outros), com a regra do `portable.txt`.
- **Ports nativos de código aberto** conhecidos (ex.: Ship of Harkinian, 2 Ship 2 Harkinian).
- **Emuladores:** Project64, RetroArch, Dolphin, PCSX2, DuckStation, PPSSPP, RPCS3, Cemu e Ryujinx.
  - Cada jogo do emulador vira um item próprio quando o nome do arquivo identifica o jogo: no Project64, o nome interno da ROM; no RetroArch, o nome da ROM; no Dolphin, PPSSPP e RPCS3, o ID do jogo.
  - Um cartão de memória compartilhado (PCSX2) vira um item só: "PCSX2 · Memory Card 1".
- O catálogo público só traz emuladores e projetos abertos e conhecidos. Qualquer outro programa entra como jogo adicionado à mão (§7.4).

### 7.4 Jogos adicionados à mão e modo Detetive

**Adicionar jogo:** nome, pasta(s) ou arquivos do save (com padrões tipo `*.sav`), executável ou nome do processo (opcional, para saber quando está aberto) e capa (opcional).

**Detetive,** para quando o usuário não sabe onde o save fica:

1. O usuário clica em "Começar" e abre o jogo.
2. O app vigia, temporariamente:
   - `%APPDATA%`, `%LOCALAPPDATA%` e `LocalLow`;
   - Documentos e Saved Games;
   - `%PUBLIC%\Documents` e `%PROGRAMDATA%`;
   - as pastas de jogos do usuário e qualquer pasta extra que ele escolher.
3. O usuário salva no jogo e clica em "Pronto".
4. O app mostra o que mudou nesse intervalo:
   - agrupado por pasta;
   - os mais prováveis primeiro (nomes com save, slot ou profile; extensões .sav, .dat, .bin, .srm…);
   - sem o ruído (temp, cache, logs, navegadores, shaders e o próprio cofre).
5. O usuário escolhe a pasta, e ela vira um jogo adicionado à mão.

### 7.5 Resultado do escaneamento

Cada jogo encontrado tem:

- id estável e título;
- origem: Steam, Epic, GOG, catálogo, manual ou só-base (jogo desinstalado com save no AppData);
- pastas de instalação;
- **raízes de save**: cada uma com o caminho real e o modelo com placeholders;
- arquivos (tamanho e data), data do último save e tamanho total;
- app ID da Steam, se houver;
- se o jogo está aberto agora;
- avisos (§15).

E um destes status:

| Status | Quando |
|---|---|
| **Protegido** | O save atual é idêntico a algum backup do cofre (comparação por conteúdo, feito em qualquer PC). |
| **Save novo** | O save atual não bate com nenhum backup (o automático vai pegar). |
| **Sem backup** | O jogo nunca teve backup. |
| **Sem save** | Jogo conhecido, sem arquivo de save. Aparece esmaecido, no fim da lista. |

O escaneamento roda em segundo plano, em paralelo, com progresso. O resultado fica em cache para a biblioteca abrir na hora. Ele é refeito ao abrir o app, ao clicar em "Escanear" e a cada 6 horas.

**"Jogando agora":** enumerar processos e casar pelo nome do executável (catálogo e manuais) ou pelo caminho do executável dentro da pasta de instalação conhecida (jogos de PC).

---

## 8. O cofre

### 8.1 Formato

```
cofre/
  cofre.json                                 versão do formato e id do cofre
  The Legend of Zelda - Ocarina of Time/     uma pasta por jogo (nome sem caracteres inválidos)
    jogo.json                                id, título, origem, modelos de caminho (e a definição, se for jogo à mão)
    2026-11-24_10-02-11_DESKTOP-7Q2K_manual.zip
    2026-11-24_19-05-40_DESKTOP-7Q2K_auto.zip
    2026-11-25_08-00-03_NOTEBOOK-3F9A_antes-de-restaurar.zip
```

- **Um `.zip` por backup**, com compressão Deflate (abre no Explorer do Windows).
- Dentro do zip: `save-control.json` (metadados) e `arquivos/<raiz>/<caminho relativo>`.
- Metadados:
  - id do jogo, título e tipo (`manual`, `auto` ou `antes-de-restaurar`);
  - nome dado pelo usuário, data e versão do app;
  - identidade do PC: um id estável (derivado do `MachineGuid` do Windows mais o usuário do Windows), o nome do PC e o nome do usuário;
  - para cada raiz: o modelo com placeholders e o caminho real;
  - para cada arquivo: caminho relativo, tamanho, data de modificação e hash BLAKE3.
- O nome do arquivo leva um rótulo curto do PC (nome do PC + 4 caracteres do id), para dois PCs nunca colidirem.
- **A fonte da verdade são os zips.** O índice que acelera a listagem é um cache local de cada PC (em `SaveControl-dados/indice/`) e pode ser reconstruído lendo os zips.
- Dentro do cofre, cada PC só **cria arquivos novos**. O único arquivo compartilhado que muda é o `jogo.json` (raramente: título, capa, definição); vale o último que gravou.
- Se dois jogos derem o mesmo nome de pasta, acrescentar um sufixo curto do id.

### 8.2 Tipos de backup e retenção

| Tipo | Limpeza automática |
|---|---|
| **Manual** (nome opcional, estrela no histórico) | Nunca é apagado automaticamente. |
| **Automático** | Por jogo **e por PC**: mantém os 10 mais recentes, mais 1 por dia dos últimos 7 dias, mais 1 por semana das últimas 4 semanas. Ajustável. Um PC nunca apaga backups de outro. |
| **Antes de restaurar** | Mantém os 5 mais recentes por jogo e por PC. |

- Não cria backup se o save atual for idêntico a algum backup que já existe (comparação por hashes).
- O usuário pode fixar (estrela) um automático, que vira permanente.
- O usuário pode apagar qualquer backup à mão, com confirmação.

### 8.3 Escrita segura

- Grava em `*.zip.parcial` dentro da pasta do jogo no cofre, fecha, relê e confere (lista de arquivos, CRC e hashes). Só então renomeia para `.zip`.
- Lê os arquivos do save com compartilhamento de leitura. Se um arquivo mudar durante a cópia (tamanho ou data diferentes antes e depois), tenta de novo após alguns segundos, até 3 vezes. Se não estabilizar, desiste desse backup e registra na Atividade.
- Confere o espaço livre antes de gravar. Ao abrir, o app limpa só as sobras `*.parcial` deste PC, ou de qualquer PC se tiverem mais de 24 h (outro PC pode estar gravando agora).

---

## 9. Backup automático

**O que dispara:**

- O app vigia, com eventos do sistema de arquivos e debounce, as raízes de save de todos os jogos com save. Uma varredura leve a cada 10 minutos cobre eventos perdidos.
- Quando algo muda, o jogo vira "Save novo" e o backup é agendado.

**Quando o backup acontece:**

| Situação | Momento do backup |
|---|---|
| Dá pra saber que o jogo está aberto | Quando o jogo **fecha**, mais 10 s |
| Jogo aberto por muito tempo | No máximo 1 backup a cada 15 minutos, para proteger sessões longas |
| Não dá pra saber se está aberto | Depois de 60 s sem mudanças |

**Notificação** nativa do Windows só quando o jogo fecha e o backup foi feito ("Backup automático feito · Ocarina of Time"), com a capa, o texto e os botões "Ver histórico" e "Dispensar".

- Vários jogos de uma vez viram uma notificação só.
- Nada de notificação durante o jogo. Além disso, o próprio Windows silencia avisos durante jogos em tela cheia.
- As notificações só funcionam depois que o usuário as liga (§13). Desligadas, o app mostra o aviso dentro da janela (se estiver aberta) e no ícone da bandeja.

**Pausar:** pela bandeja (30 min, 2 h ou até reabrir) e por jogo (aba Ajustes do jogo).

**Cofre inacessível** (ex.: HD desconectado): o automático pausa, e o ícone da bandeja e um aviso na biblioteca mostram isso. O app nunca grava em outro lugar sem perguntar. Retoma sozinho quando o cofre volta.

---

## 10. Restaurar

1. O usuário escolhe uma versão no histórico, ou em "Restaurar…".
2. Aparece a confirmação do mockup: qual versão, quantos arquivos, tamanho e destino.
3. Se o jogo estiver aberto, a restauração fica bloqueada e o app pede para fechar o jogo (botão "Tentar de novo").
4. O app cria um backup "antes de restaurar" do estado atual, se ele for diferente do último backup.
5. A restauração é **exata**, arquivo por arquivo, e só dentro dos arquivos do jogo:
   - cada arquivo é gravado num temporário e trocado pelo original (troca atômica no mesmo disco);
   - arquivos do jogo que existem agora, mas não existiam naquela versão, são removidos (eles estão guardados no backup do passo 4);
   - nunca apaga nada fora do conjunto de arquivos do jogo. Isso é importante em pastas de emulador, que guardam saves de vários jogos.
6. O app confere os hashes do que gravou. Se algo falhar no meio, desfaz usando o backup do passo 4 e mostra o erro.
7. Notificação: "Restaurado. Seu save anterior está no histórico como 'Antes de restaurar'."

Também dá para restaurar só alguns arquivos (aba Arquivos) e restaurar para outra pasta.

---

## 11. Levar para outro PC

- Cada raiz guarda o modelo com placeholders (ex.: `<winLocalAppData>/Hogwarts Legacy/Saved/SaveGames/<storeUserId>`) e o caminho real.
- Na restauração, o modelo é resolvido no PC atual: usuário, Documentos, Steam, pasta do jogo.
- Se der mais de um destino possível (ex.: duas contas da Steam) ou nenhum, o app pergunta onde restaurar e lembra a resposta.
- Em jogos manuais ou portáteis (save dentro da pasta do jogo), o modelo usa a pasta do jogo (ex.: `<pastaDoJogo>/saves`). No PC novo, o usuário aponta a pasta do jogo uma vez.
- Abrir um cofre existente (ex.: plugar o HD num PC novo) mostra os jogos do cofre mesmo que não estejam instalados, com "Restaurar…" disponível.
- **Vários PCs no mesmo cofre** (HD que vai e volta, ou cofre numa pasta do OneDrive) é suportado, até ao mesmo tempo:
  - cada PC só cria os próprios arquivos (§8.1) e tem retenção própria (§8.2);
  - o histórico mostra de qual PC veio cada backup (selo com o nome do PC);
  - restaurar um backup feito em outro PC é o jeito de "continuar de onde parei";
  - não existe mesclagem automática de progresso entre PCs.
- Arquivos do cofre que estão "só na nuvem" (OneDrive) são baixados quando forem abertos. Cópias de conflito criadas pelo serviço de nuvem são ignoradas e aparecem como aviso na tela Cofre.

---

## 12. Interface (direção A + painel lateral)

**Referência visual:** os mockups aprovados (direção A e painel lateral).

**Tokens de cor:**

| Uso | Cor |
|---|---|
| Fundo | `#0a0d13` |
| Painel | `#11151d` |
| Linhas | `rgba(255,255,255,.065)` |
| Texto (principal, secundário, terciário) | `#e9edf5`, `#a3acbd`, `#6b7489` |
| Destaque verde-água | `#3ee6a8` → `#19c6d8` |
| Aviso | `#ffb547` |
| Informação | `#5aa9ff` |

**Fontes e ícones:** Inter (texto) e Sora (títulos), **embutidas** no app (sem carregar do Google Fonts). Ícones no estilo Lucide.

### Janela

- Sem a moldura do Windows: barra de título própria, arrastável, com minimizar, maximizar e fechar.
- Tamanho mínimo 1100×700. Lembra a posição e o tamanho.

### Biblioteca (tela inicial)

- **Lateral:**
  - navegação: Biblioteca, Atividade, Cofre, Detetive, Configurações;
  - "Fontes" com contagem (Steam, Epic, Recomps & Ports, Emuladores…); clicar filtra;
  - cartão do cofre: pasta, espaço usado, número de versões e a chave do backup automático, com o ponto pulsando.
- **Topo:** busca (Ctrl+K), Escanear, Adicionar jogo e **Backup de tudo** (faz backup manual de todos os jogos com save novo).
- **Filtros:** Todos, PC, Recomps & Ports, Emuladores, Precisa de atenção.
- **Aviso no topo** quando algo pede ação: jogo sem backup, save em pasta arriscada, cofre indisponível.
- **Grade de capas** (5 colunas a 1280 px, responsiva):
  - cada cartão tem selo de status, nome e "origem · quando";
  - no hover: "Fazer backup" e "Restaurar…";
  - ordem padrão pelo último save; os "Sem save" vão para o fim, esmaecidos.
- **Capas:**
  - arte vertical oficial da Steam (600×900), pelo app ID, quando houver; baixada uma vez e guardada em `capas/`;
  - senão, uma **capa gerada**: gradiente, emblema e título, sempre igual para o mesmo jogo (como nos mockups);
  - o usuário pode escolher uma imagem.

### Painel lateral (ao clicar num jogo)

Desliza da direita (~600 px) sobre a biblioteca escurecida. Fecha com Esc ou clicando fora.

- **Topo:** capa com selo de status e botão de fechar; origem, título e chips (jogando agora, número de arquivos ou slots, tamanho).
- **Ações:**
  - Fazer backup (pergunta um nome, opcional);
  - Restaurar…;
  - Abrir pasta;
  - menu ⋯: renomear, trocar capa, ignorar jogo, remover jogo manual.
- **Avisos** do jogo (§15).
- **Abas:**
  - **Histórico:** linha do tempo com o save atual e os backups; estrela nos backups com nome; "Restaurar" no hover;
  - **Arquivos:** por raiz, com restauração seletiva;
  - **Ajustes:** backup automático deste jogo, retenção, incluir configurações.
- **Rodapé:** o caminho do save e "Abrir pasta".

### Outras telas

- **Atividade:** linha do tempo de backups, restaurações e erros, com filtro.
- **Cofre:** local, espaço por jogo, mover o cofre (com cópia segura), verificar integridade, abrir a pasta.
- **Detetive:** o passo a passo do §7.4.
- **Configurações:**
  - pasta do cofre;
  - backup automático, tempos e retenção;
  - iniciar com o Windows e fechar para a bandeja;
  - notificações;
  - incluir configurações;
  - pastas de jogos;
  - atualizar a base e baixar capas oficiais;
  - sobre e créditos.

### Primeiro uso

1. Boas-vindas.
2. Escolher onde fica o cofre (padrão: ao lado do app).
3. Escaneamento, com progresso animado.
4. Resumo ("Encontramos 38 jogos com save").
5. "Fazer o primeiro backup de tudo" (recomendado).
6. Ligar o backup automático e "iniciar com o Windows".

### Peças fixas

- A **confirmação de restaurar** faz parte da janela e segue exatamente o mockup.
- A **notificação de backup automático** e o **menu da bandeja** são nativos do Windows: mesmo conteúdo dos mockups (textos, capa, botões e itens), mas com o visual do sistema, que acompanha o tema claro/escuro do Windows. O cabeçalho do menu ("Save Control · Vigiando 38 jogos") vira um item informativo desabilitado.

### Movimento e acessibilidade

- Animações curtas (150–250 ms), respeitando a opção de reduzir animações do Windows.
- Tudo alcançável pelo teclado, com foco visível e contraste AA nos textos.

---

## 13. Segundo plano

- **Bandeja:**
  - o ícone mostra o estado: normal, fazendo backup, pausado ou cofre indisponível;
  - menu nativo, com os itens do mockup: Abrir, Backup de tudo agora, Pausar ›, Abrir pasta do cofre, Sair.
- **Notificações do Windows:**
  - um app portátil precisa se registrar no usuário do Windows para mostrar notificações (um AppUserModelID em `HKCU\Software\Classes\AppUserModelId\…`, com nome e ícone);
  - esse registro só é feito quando o usuário liga as notificações (passo do primeiro uso ou Configurações);
  - os botões da notificação funcionam enquanto o app estiver rodando, que é o normal, já que ele vive na bandeja;
  - clicar no aviso abre o painel do jogo.
- **Fechar a janela** = ir para a bandeja (ajustável). A WebView é destruída para liberar memória.
- **Iniciar com o Windows:** entrada em `HKCU\Software\Microsoft\Windows\CurrentVersion\Run` apontando para o `.exe` atual com o argumento de abrir na bandeja. Só é criada quando o usuário liga. Se o `.exe` mudar de lugar, o app corrige a entrada ao abrir; se o caminho antigo era um HD removível, avisa.
- **Instância única.**
- **Remover registros do Windows:** botão em Configurações que apaga a entrada de iniciar com o Windows e o registro das notificações. Também é oferecido quando o usuário quiser apagar o app.

---

## 14. Desempenho

- Biblioteca visível em menos de 1,5 s, abrindo com cache.
- Escaneamento completo em menos de 15 s num PC comum.
- Na bandeja: CPU perto de 0% quando parado e pouca memória (sem WebView).
- Backups com prioridade baixa de disco, e nunca durante o jogo, exceto o de 15 em 15 minutos.

---

## 15. Avisos e casos-limite

| Situação | O que o app faz |
|---|---|
| Save em pasta arriscada (Downloads, Área de Trabalho, Temp) | Aviso no jogo e no topo da biblioteca |
| Save dentro da pasta do jogo (modo portátil) | Aviso: "atualizar apagando a pasta do jogo apaga o save; faça um backup antes" |
| Jogo com nuvem da loja (campo `cloud`) | Selo informativo: "também tem nuvem da Steam" |
| Jogo com dados no registro | Aviso: "dados no registro não incluídos" |
| Caminhos com mais de 260 caracteres ou caracteres especiais | Usar APIs com suporte a caminho longo |
| Arquivo travado pelo jogo | Tentar de novo; se persistir, registrar e avisar sem travar o resto |
| Cofre em pendrive ou HD com exFAT/FAT32 | Funciona (não depende de hardlinks); avisar se algum arquivo passar de 4 GB em FAT32 |
| Dois PCs (ou duas contas do Windows) usando o mesmo cofre, até ao mesmo tempo | Suportado (§11): cada um grava só os próprios arquivos, com identidade própria |

---

## 16. Testes

**`core` com testes automáticos:**

- expansão de placeholders e globs;
- leitura da base (amostra fixa no repositório);
- Steam (`libraryfolders.vdf`, `.acf`), Epic (`.item`) e GOG (registro simulado);
- regras do catálogo, com e sem `portable.txt`;
- escrita e verificação do zip;
- retenção;
- restauração exata, incluindo arquivos extras e pasta de emulador compartilhada;
- tradução de caminhos entre "PCs" simulados;
- dois PCs simulados no mesmo cofre: nomes de arquivo, retenção por PC, status por conteúdo e limpeza de `.parcial`;
- Detetive: filtro de ruído e ordenação.

**Ponta a ponta,** com uma pasta "home" falsa: escanear → backup → mudar o save → backup automático → restaurar → conferir os hashes.

**Interface:** conferência visual com o app rodando, comparando com os mockups.

**Aceitação no PC do usuário:**

- um jogo da base (ex.: Hogwarts Legacy);
- um recomp em modo portátil (Banjo Recompiled);
- um jogo adicionado pelo Detetive;
- uma restauração de ida e volta, feita numa cópia.

---

## 17. Build e distribuição

- `tauri build` sem instalador gera o `SaveControl.exe`. Distribuir como `SaveControl-vX.Y.Z-portatil.zip` (exe + LEIAME).
- GitHub Actions (windows-latest) para gerar o zip quando houver tag. Opcional na v1.
- Sem assinatura de código: o SmartScreen vai avisar na primeira vez.

---

## 18. Créditos e licenças

- **Base de locais de save:** ludusavi-manifest (dados compilados do PCGamingWiki e da API da Steam). Dar crédito no "Sobre" e no README. Conferir os termos da licença dos dados do PCGamingWiki antes de publicar versões.
- **Inspiração:** Ludusavi (licença MIT). Se algum trecho de código for adaptado, manter o aviso MIT.
- **Fontes e ícones:** Inter e Sora (OFL) e ícones Lucide (ISC). Incluir as licenças.
- **Licença do Save Control:** a definir pelo usuário. Sugestão: MIT.

---

## 19. Fica a critério de quem implementar

- Estrutura de componentes React, gerência de estado e biblioteca de animação.
- Formato do cache pré-processado da base.
- Detalhes de cada entrada do catálogo: caminhos exatos e como separar por jogo.
- Heurísticas finas do Detetive.
- Ordem da implementação. Sugestão:
  1. núcleo, cofre e backup/restauração manual;
  2. biblioteca e painel lateral;
  3. backup automático e bandeja;
  4. Detetive, capas e catálogo de emuladores;
  5. primeiro uso e polimento.

---

## 20. Quem decidiu o quê

**Decidido pelo usuário:**

- App de Windows, portátil, com visual moderno e profissional.
- Os saves ficam centralizados num cofre que dá pra levar num HD.
- Restaurar substitui o save que o jogo cria.
- Escopo completo: jogos de PC (Steam, Epic…), recomps, ports, emuladores e jogos adicionados à mão.
- Backup automático em segundo plano, além do manual.
- Nome "Save Control" e repositório público no GitHub.
- Feito em Rust, com visual muito mais bonito que o do Ludusavi.
- Direção visual A, com detalhe em painel lateral.
- Nenhuma atribuição a IA nos commits.

**Decisões do autor do design (o usuário delegou ou não especificou):**

1. **Tauri 2 + React + TypeScript** (o usuário delegou).
2. **Cofre** com um `.zip` por backup, que abre no Explorer. Seguro para vários PCs, até ao mesmo tempo: cada PC grava só os próprios arquivos e tem retenção própria.
3. **Retenção dos automáticos:** os 10 mais recentes + 1 por dia (7 dias) + 1 por semana (4 semanas). Backups com nome nunca são apagados; "antes de restaurar" guarda os 5 últimos.
4. **Momento do automático:** quando o jogo fecha, quando dá pra saber que ele está aberto; senão, após 60 s sem mudanças. No máximo 1 a cada 15 min durante o jogo.
5. **Restauração exata:** arquivos que surgiram depois do backup são removidos, mas ficam guardados no "antes de restaurar".
6. **Fechar a janela** manda pra bandeja; "iniciar com o Windows" é oferecido no primeiro uso, desligado até o usuário ligar.
7. **Primeiro uso precisa de internet** para baixar a base de jogos de PC (17 MB); depois funciona offline e atualiza uma vez por dia.
8. **Capas oficiais da Steam** quando houver; senão, capas geradas.
9. **Fora da v1:** só Windows; sem nuvem; sem saves no registro; sem saves da Microsoft Store/Xbox.
10. **Interface só em pt-BR** na v1, com os textos centralizados.
11. **O catálogo público** só inclui emuladores e projetos abertos e conhecidos; outros programas entram como jogo adicionado à mão (com o Detetive).
12. **Cofre indisponível** pausa o automático e avisa; o app nunca grava em outro lugar sozinho.
13. **Menu da bandeja e notificação nativos do Windows,** com o conteúdo dos mockups, mas sem o visual idêntico. As notificações só funcionam depois que o usuário as liga, porque exigem um registro no usuário do Windows (removível).
