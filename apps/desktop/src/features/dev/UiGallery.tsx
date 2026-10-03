import { useState, type ReactNode } from "react";
import { Gamepad2, Plus, Settings, Trash2 } from "lucide-react";
import { Badge } from "@/components/ui/badge";
import { Button, IconButton } from "@/components/ui/button";
import { Card, CardDescription, CardTitle } from "@/components/ui/card";
import {
  Dialog,
  DialogClose,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogTitle,
  DialogTrigger,
} from "@/components/ui/dialog";
import { EmptyState } from "@/components/ui/empty-state";
import { Input } from "@/components/ui/input";
import { Kbd } from "@/components/ui/kbd";
import { SegmentedControl } from "@/components/ui/segmented-control";
import { Select } from "@/components/ui/select";
import { Skeleton } from "@/components/ui/skeleton";
import { Slider } from "@/components/ui/slider";
import { Stepper } from "@/components/ui/stepper";
import { Toggle } from "@/components/ui/toggle";
import { Tooltip } from "@/components/ui/tooltip";
import { applyAccent, applyTheme, DEFAULT_ACCENT, type Theme } from "@/lib/theme";
import { useToasts } from "@/lib/toast";
import { cn } from "@/lib/utils";

const ACCENTS = [DEFAULT_ACCENT, "#30D158", "#FF9F0A", "#BF5AF2"] as const;
const STEPS = ["Boas-vindas", "Driver", "Firewall", "Parear", "Pronto"] as const;
const LANGUAGES = [
  { value: "pt-BR", label: "Português (Brasil)" },
  { value: "en", label: "English" },
] as const;

function Section({ title, children }: { title: string; children: ReactNode }) {
  return (
    <section className="flex flex-col gap-3">
      <h2 className="text-caption font-semibold tracking-wide text-fg-3 uppercase">{title}</h2>
      <div className="flex flex-wrap items-center gap-3">{children}</div>
    </section>
  );
}

/** Todos os componentes de um tema; o painel inteiro recebe o `data-theme` pedido. */
function Panel({ theme }: { theme: Theme }) {
  const [segment, setSegment] = useState("grid");
  const [language, setLanguage] = useState("pt-BR");
  const [deadzone, setDeadzone] = useState(15);
  const [enabled, setEnabled] = useState(true);
  const [name, setName] = useState("");
  const push = useToasts((s) => s.push);

  return (
    <div
      data-theme={theme}
      className="flex min-w-0 flex-1 flex-col gap-6 rounded-xl bg-window p-6 text-fg"
    >
      <h1 className="text-title font-semibold">{theme === "dark" ? "Dark" : "Light"}</h1>

      <Section title="Botões">
        <Button variant="primary">Primário</Button>
        <Button variant="secondary">Secundário</Button>
        <Button variant="ghost">Ghost</Button>
        <Button variant="destructive">
          <Trash2 /> Remover
        </Button>
        <Button variant="primary" disabled>
          Desativado
        </Button>
        <IconButton label="Configurações">
          <Settings strokeWidth={1.75} />
        </IconButton>
      </Section>

      <Section title="Seleção">
        <Toggle aria-label="Ativado" checked={enabled} onCheckedChange={setEnabled} />
        <SegmentedControl
          label="Visualização"
          value={segment}
          onValueChange={setSegment}
          options={[
            { value: "grid", label: "Grade" },
            { value: "list", label: "Lista" },
            { value: "compact", label: "Compacta" },
          ]}
        />
        <Select label="Idioma" value={language} onValueChange={setLanguage} options={LANGUAGES} />
      </Section>

      <Section title="Slider">
        <Slider
          className="w-full"
          label="Zona morta"
          value={deadzone}
          onValueChange={setDeadzone}
          min={0}
          max={50}
          defaultValue={15}
          format={(v) => `${v}%`}
        />
      </Section>

      <Section title="Campo">
        <Input
          aria-label="Nome do dispositivo"
          placeholder="Nome do dispositivo"
          value={name}
          onChange={(e) => {
            setName(e.target.value);
          }}
          className="max-w-64"
        />
        <Input
          aria-label="Campo inválido"
          aria-invalid
          defaultValue="valor inválido"
          className="max-w-64"
        />
      </Section>

      <Section title="Badges e atalhos">
        <Badge status="success">Conectado</Badge>
        <Badge status="warning">Atenção</Badge>
        <Badge status="danger">Offline</Badge>
        <Badge status="info">Pareando</Badge>
        <Badge>Ocioso</Badge>
        <span className="flex items-center gap-1 text-fg-2">
          <Kbd>Ctrl</Kbd>
          <Kbd>K</Kbd>
        </span>
      </Section>

      <Section title="Cores dos slots">
        {(["bg-slot-1", "bg-slot-2", "bg-slot-3", "bg-slot-4"] as const).map((c, i) => (
          <span key={c} className="flex items-center gap-2 text-fg-2">
            <span className={cn("size-3 rounded-full", c)} /> P{i + 1}
          </span>
        ))}
      </Section>

      <Section title="Card">
        <Card className="w-full">
          <CardTitle>Controle do João</CardTitle>
          <CardDescription>Pixel 8 · 12 ms · bateria 84%</CardDescription>
        </Card>
      </Section>

      <Section title="Stepper">
        <Stepper steps={STEPS} current={2} className="flex-wrap" />
      </Section>

      <Section title="Overlays">
        <Dialog>
          <DialogTrigger asChild>
            <Button>Abrir dialog</Button>
          </DialogTrigger>
          <DialogContent>
            <DialogTitle>Apagar perfil?</DialogTitle>
            <DialogDescription>Esta ação não pode ser desfeita.</DialogDescription>
            <DialogFooter>
              <DialogClose asChild>
                <Button>Cancelar</Button>
              </DialogClose>
              <DialogClose asChild>
                <Button variant="destructive">Apagar</Button>
              </DialogClose>
            </DialogFooter>
          </DialogContent>
        </Dialog>
        <Dialog>
          <DialogTrigger asChild>
            <Button>Abrir sheet</Button>
          </DialogTrigger>
          <DialogContent placement="sheet">
            <DialogTitle>Detalhes</DialogTitle>
            <DialogDescription>Painel lateral com blur no fundo.</DialogDescription>
          </DialogContent>
        </Dialog>
        <Tooltip content="Dica de ajuda">
          <Button>Tooltip</Button>
        </Tooltip>
        <Button
          onClick={() => {
            push("success", "Perfil salvo");
          }}
        >
          Toast
        </Button>
        <Button
          onClick={() => {
            push("error", "Não consegui salvar o perfil");
          }}
        >
          Toast de erro
        </Button>
      </Section>

      <Section title="Vazio e carregando">
        <Card className="flex-1">
          <EmptyState
            icon={<Gamepad2 strokeWidth={1.75} />}
            title="Nenhum controle conectado"
            description="Abra o app no celular e escaneie o QR code para parear."
            action={
              <Button variant="primary">
                <Plus /> Parear novo
              </Button>
            }
          />
        </Card>
        <div className="flex w-full flex-col gap-2">
          <Skeleton className="h-4 w-1/2" />
          <Skeleton className="h-4 w-3/4" />
        </div>
      </Section>
    </div>
  );
}

/** `/dev/ui`: vitrine dos componentes nos dois temas (só existe em dev). */
export function UiGallery() {
  const [theme, setTheme] = useState<Theme>("dark");
  const [accent, setAccent] = useState<string>(DEFAULT_ACCENT);

  return (
    <div className="flex flex-col gap-4 p-6">
      <div className="flex flex-wrap items-center gap-4">
        <SegmentedControl
          label="Tema do app"
          value={theme}
          onValueChange={(t) => {
            setTheme(t);
            applyTheme(t);
          }}
          options={[
            { value: "dark", label: "Dark" },
            { value: "light", label: "Light" },
          ]}
        />
        <div role="group" aria-label="Accent" className="flex gap-2">
          {ACCENTS.map((c) => (
            <button
              key={c}
              type="button"
              aria-label={`Accent ${c}`}
              aria-pressed={accent === c}
              onClick={() => {
                setAccent(applyAccent(c));
              }}
              className="size-6 rounded-full ring-2 ring-transparent focus-ring aria-pressed:ring-fg"
              style={{ backgroundColor: c }}
            />
          ))}
        </div>
      </div>
      <div className="flex flex-col gap-4 xl:flex-row">
        <Panel theme="dark" />
        <Panel theme="light" />
      </div>
    </div>
  );
}
