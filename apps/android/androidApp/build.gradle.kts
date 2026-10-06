plugins {
    alias(libs.plugins.android.application)
    alias(libs.plugins.kotlin.compose)
    alias(libs.plugins.compose.multiplatform)
}

val ndkVersion = "27.1.12297006"
val androidHome = System.getenv("ANDROID_HOME") ?: System.getenv("ANDROID_SDK_ROOT")
    ?: error("Defina ANDROID_HOME apontando para o Android SDK")

val buildCoreAndroid = tasks.register<CargoNdkBuild>("buildCoreAndroid") {
    workspaceDir.set(rootProject.layout.projectDirectory.dir("../.."))
    abis.set(listOf("arm64-v8a", "armeabi-v7a", "x86_64"))
    ndkDir.set("$androidHome/ndk/$ndkVersion")
    jniLibsDir.set(layout.buildDirectory.dir("rustJniLibs"))
    doNotTrackState("O cargo faz o próprio controle de cache")
}

android {
    ndkVersion = "27.1.12297006"
    namespace = "app.vynpad.android"
    compileSdk = 37

    defaultConfig {
        applicationId = "app.vynpad.android"
        minSdk = 26
        targetSdk = 36
        versionCode = 1
        versionName = "0.1.0"
    }

    // Assinatura de release: o keystore e as senhas vêm do ambiente (CI ou máquina de quem
    // publica), nunca do repositório. Sem elas o release sai sem assinar, e o Play App Signing
    // assina o bundle de upload com a chave de envio.
    val uploadKeystore = System.getenv("VYNPAD_KEYSTORE")
    if (uploadKeystore != null) {
        signingConfigs.create("upload") {
            storeFile = file(uploadKeystore)
            storePassword = System.getenv("VYNPAD_KEYSTORE_PASSWORD")
            keyAlias = System.getenv("VYNPAD_KEY_ALIAS")
            keyPassword = System.getenv("VYNPAD_KEY_PASSWORD")
        }
    }

    buildTypes {
        release {
            signingConfigs.findByName("upload")?.let { signingConfig = it }
            isMinifyEnabled = true
            isShrinkResources = true
            proguardFiles(getDefaultProguardFile("proguard-android-optimize.txt"), "proguard-rules.pro")
        }
    }

    sourceSets.getByName("main").jniLibs.directories.add(
        layout.buildDirectory.dir("rustJniLibs").get().asFile.path,
    )

    lint {
        abortOnError = true
        // Avisos não derrubam o build; erros sim. targetSdk 36 é deliberado (a 37 ainda é prévia no Play).
        warningsAsErrors = false
        // O ícone adaptativo exige a pasta -v26 mesmo com minSdk 26 (o AAPT não o acha sem ela).
        disable += listOf("OldTargetApi", "ObsoleteSdkInt")
    }

    compileOptions {
        sourceCompatibility = JavaVersion.VERSION_17
        targetCompatibility = JavaVersion.VERSION_17
    }
}

dependencies {
    implementation(project(":shared"))
    implementation(libs.androidx.activity.compose)
}

// A biblioteca nativa precisa existir antes de o AGP juntar as .so no APK.
tasks.configureEach {
    if (name.startsWith("merge") && name.endsWith("JniLibFolders")) dependsOn(buildCoreAndroid)
}
