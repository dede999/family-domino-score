# As classes da WebView são chamadas pelo Rust (JNI), por nome; o
# encolhimento da versão release não pode renomeá-las nem apagá-las.
-keep class dev.dioxus.main.** { *; }
