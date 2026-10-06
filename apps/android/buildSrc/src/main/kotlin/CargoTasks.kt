import javax.inject.Inject
import org.gradle.api.DefaultTask
import org.gradle.api.file.DirectoryProperty
import org.gradle.api.provider.ListProperty
import org.gradle.api.provider.Property
import org.gradle.api.tasks.Input
import org.gradle.api.tasks.Internal
import org.gradle.api.tasks.Optional
import org.gradle.api.tasks.OutputDirectory
import org.gradle.api.tasks.TaskAction
import org.gradle.process.ExecOperations

/** Roda um comando do cargo na raiz do workspace Rust; o cargo decide o que está desatualizado. */
abstract class RunCargo : DefaultTask() {
    @get:Inject
    abstract val execOps: ExecOperations

    @get:Internal
    abstract val workspaceDir: DirectoryProperty

    @get:Input
    abstract val arguments: ListProperty<String>

    @get:OutputDirectory
    @get:Optional
    abstract val outputDir: DirectoryProperty

    @TaskAction
    fun run() {
        outputDir.orNull?.asFile?.mkdirs()
        execOps.exec {
            workingDir = workspaceDir.get().asFile
            commandLine(listOf("cargo") + arguments.get())
        }
    }
}

/** Compila o núcleo Rust para os ABIs do Android com o `cargo-ndk`. */
abstract class CargoNdkBuild : DefaultTask() {
    @get:Inject
    abstract val execOps: ExecOperations

    @get:Internal
    abstract val workspaceDir: DirectoryProperty

    @get:Input
    abstract val abis: ListProperty<String>

    @get:Input
    abstract val ndkDir: Property<String>

    @get:OutputDirectory
    abstract val jniLibsDir: DirectoryProperty

    @TaskAction
    fun run() {
        val targets = abis.get().flatMap { listOf("-t", it) }
        execOps.exec {
            workingDir = workspaceDir.get().asFile
            environment("ANDROID_NDK_HOME", ndkDir.get())
            commandLine(
                listOf("cargo", "ndk") + targets +
                    listOf("-o", jniLibsDir.get().asFile.absolutePath, "build", "-p", "vynpad-client-core", "--release", "--locked"),
            )
        }
    }
}
