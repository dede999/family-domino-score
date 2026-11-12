// Atividade do app Android (ver `[application] android_main_activity` no
// Dioxus.toml). Igual à que o dx gera, mais o espaço das barras do sistema:
// com o targetSdk 35 ou mais, o Android desenha o app na tela inteira, e o
// topo da página ficava embaixo da barra de status (e os campos, embaixo do
// teclado).
package dev.dioxus.main

import android.content.res.Configuration
import android.graphics.Color
import android.os.Bundle
import android.view.View
import androidx.core.view.ViewCompat
import androidx.core.view.WindowCompat
import androidx.core.view.WindowInsetsCompat

typealias BuildConfig = com.santamariasoftware.placar.BuildConfig

class MainActivity : WryActivity() {
    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)

        // Atrás das barras aparece o fundo da janela: a mesma cor do fundo
        // da página (`--bg` em assets/main.css), clara ou escura conforme o
        // tema do celular.
        val night = (resources.configuration.uiMode and Configuration.UI_MODE_NIGHT_MASK) ==
            Configuration.UI_MODE_NIGHT_YES
        window.decorView.setBackgroundColor(Color.parseColor(if (night) "#151412" else "#f4f1ea"))
        WindowCompat.getInsetsController(window, window.decorView).apply {
            isAppearanceLightStatusBars = !night
            isAppearanceLightNavigationBars = !night
        }

        val content = findViewById<View>(android.R.id.content)
        ViewCompat.setOnApplyWindowInsetsListener(content) { view, insets ->
            val bars = insets.getInsets(
                WindowInsetsCompat.Type.systemBars() or
                    WindowInsetsCompat.Type.displayCutout() or
                    WindowInsetsCompat.Type.ime()
            )
            view.setPadding(bars.left, bars.top, bars.right, bars.bottom)
            WindowInsetsCompat.CONSUMED
        }
    }
}
