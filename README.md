# Agenda

Aplicativo de agenda em Tauri v2 com interface Leptos compilada para WebAssembly e comandos nativos em Rust. Os dados ficam no diretório local da aplicação; lembretes são notificações locais do Android, sem servidor ou FCM.

## Recursos do MVP

- Visual semanal e eventos por dia.
- Criação e exclusão de eventos com descrição, etiquetas e antecedência do lembrete.
- Armazenamento local em `agenda.json`, no diretório de dados da aplicação.
- Notificações locais agendadas no Android, inclusive com o app fechado. O Android pode aplicar restrições de economia de bateria/Doze.
- Na versão desktop de desenvolvimento, os eventos são persistidos, mas notificações agendadas são habilitadas somente no Android.

## Pré-requisitos

- Rust estável e o alvo `wasm32-unknown-unknown`.
- Trunk (`cargo install --locked trunk`).
- Node.js e npm (`npm install`).
- Para Android: Android Studio/SDK, NDK, JDK e o Tauri CLI incluído nas dependências npm.

Configure o SDK Android no ambiente e instale os alvos usados pelo Tauri:

```sh
rustup target add wasm32-unknown-unknown aarch64-linux-android armv7-linux-androideabi i686-linux-android x86_64-linux-android
```

As flags do linker em `.cargo/config.toml` alinham as bibliotecas Rust nativas para páginas de memória de 16 KB, necessárias em dispositivos Android compatíveis com esse tamanho de página.

Inicialize a plataforma Android uma vez e execute no dispositivo/emulador:

```sh
npm run tauri android init
npm run tauri android dev
```

Para gerar o APK:

```sh
npm run tauri android build
```

Esse comando gera um APK de release sem assinatura e um AAB. Para gerar um APK de depuração assinado para instalar e testar em um aparelho ARM64:

```sh
./node_modules/.bin/tauri android build --debug --apk --target aarch64
```

Configure uma chave de assinatura antes de distribuir o APK/AAB de release. O Android perguntará pela permissão de notificações quando o primeiro lembrete for criado. Se a permissão for negada, o evento não será salvo para evitar a falsa indicação de que o aviso foi agendado.


emulador:
```sh
/your/path/to/Android/Sdk/emulator/emulator -avd Medium_Phone_API_37.0
```