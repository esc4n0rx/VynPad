import Image from "next/image";
import type { Dictionary } from "@/i18n";
import styles from "./hero-scene.module.css";
import { PhoneVideo } from "./phone-video";

interface HeroSceneProps {
  dict: Pick<Dictionary, "hero">;
}

/**
 * O produto, de verdade: a janela do VynPad no Windows (captura real) e o app no celular (vídeo
 * real do controle em uso). Os pacotes que correm entre os dois são decoração.
 */
export function HeroScene({ dict }: HeroSceneProps) {
  const scene = dict.hero.scene;
  return (
    <figure className={styles.wrap} aria-label={scene.phoneLabel}>
      <div className={styles.scene}>
        {/* Janela do desktop */}
        <div className={styles.window}>
          <Image
            src="/shots/desktop.webp"
            alt={scene.desktopAlt}
            width={830}
            height={500}
            priority
            className={styles.shot}
          />
        </div>

        {/* Pacotes do celular ao card do P1 */}
        <svg
          className={`${styles.link ?? ""} motion-only`}
          viewBox="0 0 40 31"
          aria-hidden
          focusable="false"
        >
          <path
            id="packet-route"
            className={styles.linkPath}
            d="M 13.2 19.5 C 13.4 16.6, 16 14.6, 19.4 13.4"
          />
          {[0, 0.5, 1].map((delay) => (
            <circle key={delay} className={styles.packet} r="0.28">
              <animateMotion
                dur="1.5s"
                begin={`${String(delay)}s`}
                repeatCount="indefinite"
                keyPoints="0;1"
                keyTimes="0;1"
              >
                <mpath href="#packet-route" />
              </animateMotion>
            </circle>
          ))}
        </svg>

        {/* Celular em paisagem */}
        <div className={styles.phone}>
          <div className={styles.screen}>
            <PhoneVideo
              src="/shots/phone.mp4"
              poster="/shots/phone-poster.webp"
              still="/shots/phone-live.webp"
              alt={scene.phoneAlt}
            />
          </div>
        </div>
      </div>
      <figcaption className={styles.caption}>
        <span className="sr-only">{scene.packetLabel}. </span>
        {scene.caption}
      </figcaption>
    </figure>
  );
}
