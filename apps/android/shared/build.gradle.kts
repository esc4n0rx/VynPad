plugins {
    alias(libs.plugins.kotlin.multiplatform)
    alias(libs.plugins.android.kotlin.multiplatform.library)
    alias(libs.plugins.kotlin.compose)
    alias(libs.plugins.compose.multiplatform)
}

kotlin {
    androidLibrary {
        namespace = "app.vynpad.shared"
        compileSdk = 37
        minSdk = 26

        // Necessário para os recursos do Compose (strings, fontes) irem para o APK.
        androidResources.enable = true
    }

    sourceSets {
        commonMain.dependencies {
            implementation(compose.runtime)
            implementation(compose.foundation)
            implementation(compose.ui)
            implementation(compose.components.resources)
            implementation(libs.kotlinx.coroutines.core)
        }
        androidMain.dependencies {
            implementation(libs.androidx.activity.compose)
            implementation(libs.androidx.core.ktx)
        }
    }
}

compose.resources {
    packageOfResClass = "app.vynpad.resources"
    generateResClass = always
}
