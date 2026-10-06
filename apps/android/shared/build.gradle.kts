plugins {
    alias(libs.plugins.kotlin.multiplatform)
    alias(libs.plugins.android.kotlin.multiplatform.library)
    alias(libs.plugins.kotlin.compose)
    alias(libs.plugins.kotlin.serialization)
    alias(libs.plugins.compose.multiplatform)
}

val workspaceRoot = rootProject.layout.projectDirectory.dir("../..")
val hostLibrary = System.getProperty("os.name").lowercase().let { os ->
    when {
        "win" in os -> "vynpad_client_core.dll"
        "mac" in os -> "libvynpad_client_core.dylib"
        else -> "libvynpad_client_core.so"
    }
}

// Biblioteca do computador: só serve para o gerador de bindings ler os metadados do UniFFI.
val buildCoreHost = tasks.register<RunCargo>("buildCoreHost") {
    workspaceDir.set(workspaceRoot)
    arguments.set(listOf("build", "-p", "vynpad-client-core", "--locked"))
    doNotTrackState("O cargo faz o próprio controle de cache")
}

val coreBindingsDir = layout.buildDirectory.dir("generated/uniffi/kotlin")

val generateCoreBindings = tasks.register<RunCargo>("generateCoreBindings") {
    dependsOn(buildCoreHost)
    workspaceDir.set(workspaceRoot)
    outputDir.set(coreBindingsDir)
    arguments.set(
        listOf(
            "run", "-q", "-p", "vynpad-client-core", "--features", "bindgen", "--bin", "uniffi-bindgen", "--",
            "generate", "--library", "target/debug/$hostLibrary", "--language", "kotlin", "--no-format",
            "--out-dir", coreBindingsDir.get().asFile.absolutePath,
        ),
    )
    doNotTrackState("O cargo faz o próprio controle de cache")
}

kotlin {
    androidLibrary {
        namespace = "app.vynpad.shared"
        compileSdk = 37
        minSdk = 26

        // Necessário para os recursos do Compose (strings, fontes) irem para o APK.
        androidResources.enable = true

        // Testes de lógica pura (geometria e roteamento de toques) rodam na JVM, sem aparelho.
        withHostTest { }
    }

    sourceSets {
        commonTest.dependencies {
            implementation(kotlin("test"))
        }
        commonMain.dependencies {
            implementation(compose.runtime)
            implementation(compose.foundation)
            implementation(compose.ui)
            implementation(compose.components.resources)
            implementation(libs.kotlinx.coroutines.core)
            implementation(libs.kotlinx.serialization.json)
        }
        androidMain {
            // Bindings do núcleo Rust gerados pelo UniFFI (pacote `uniffi.vynpad_client_core`).
            kotlin.srcDir(generateCoreBindings.map { it.outputDir })
            dependencies {
                implementation(libs.androidx.activity.compose)
                implementation(libs.androidx.core.ktx)
                // O UniFFI chama a biblioteca nativa por JNA; no Android vem como AAR.
                implementation("${libs.jna.get()}@aar")
                // Leitura do QR de pareamento: CameraX (preview e análise) + ML Kit (modelo embutido).
                implementation(libs.androidx.camera.camera2)
                implementation(libs.androidx.camera.lifecycle)
                implementation(libs.androidx.camera.view)
                implementation(libs.mlkit.barcode.scanning)
            }
        }
    }
}

compose.resources {
    packageOfResClass = "app.vynpad.resources"
    generateResClass = always
}
