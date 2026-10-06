# Regras do R8 para o build de release do VynPad.

# O núcleo Rust é chamado por JNA a partir dos bindings do UniFFI: o JNA acha classes, campos e
# callbacks por reflexão, então nada disso pode ser renomeado ou removido.
-keep class com.sun.jna.** { *; }
-keep class * implements com.sun.jna.** { *; }
-keep class uniffi.** { *; }
-dontwarn java.awt.**

# Os callbacks implementam interfaces do UniFFI (SessionListener, PumpListener) e são chamados
# de volta pelo código nativo.
-keepclassmembers class * implements uniffi.vynpad_client_core.SessionListener { *; }
-keepclassmembers class * implements uniffi.vynpad_client_core.PumpListener { *; }

# kotlinx.serialization: os serializadores gerados são achados pelo nome da classe.
-keepattributes *Annotation*, InnerClasses
-dontnote kotlinx.serialization.**
-keepclassmembers class app.vynpad.** {
    *** Companion;
}
-keepclasseswithmembers class app.vynpad.** {
    kotlinx.serialization.KSerializer serializer(...);
}

# ML Kit (leitor de QR): os registradores de componentes são instanciados por reflexão pelo
# serviço de descoberta; sem o construtor, o leitor falha só no release.
-keep class * implements com.google.firebase.components.ComponentRegistrar { <init>(); }
-keep class com.google.mlkit.common.internal.MlKitComponentDiscoveryService { *; }
