"use client";

import Image from "next/image";
import { useEffect, useRef, useSyncExternalStore } from "react";

interface PhoneVideoProps {
  src: string;
  /** Primeiro quadro (controle parado): aparece antes do vídeo carregar. */
  poster: string;
  /** Quadro estático usado por quem pediu menos movimento (controle em uso). */
  still: string;
  alt: string;
}

const QUERY = "(prefers-reduced-motion: reduce)";

function subscribe(onChange: () => void) {
  const media = matchMedia(QUERY);
  media.addEventListener("change", onChange);
  return () => {
    media.removeEventListener("change", onChange);
  };
}

/** Preferência por menos movimento. No servidor vale `false`; no navegador, a do sistema. */
function usePrefersReducedMotion(): boolean {
  return useSyncExternalStore(
    subscribe,
    () => matchMedia(QUERY).matches,
    () => false,
  );
}

/**
 * A tela do celular: um vídeo real do app em uso, em loop e sem som. Só toca enquanto está na
 * tela (poupa bateria e rede) e, para quem pediu menos movimento, vira uma imagem parada.
 */
export function PhoneVideo({ src, poster, still, alt }: PhoneVideoProps) {
  const reduce = usePrefersReducedMotion();
  const ref = useRef<HTMLVideoElement>(null);

  useEffect(() => {
    const video = ref.current;
    if (!video || reduce) return;
    const observer = new IntersectionObserver(
      ([entry]) => {
        if (entry?.isIntersecting) void video.play().catch(() => undefined);
        else video.pause();
      },
      { threshold: 0.25 },
    );
    observer.observe(video);
    return () => {
      observer.disconnect();
    };
  }, [reduce]);

  if (reduce) {
    return (
      <Image src={still} alt={alt} width={1600} height={720} className="size-full object-cover" />
    );
  }
  return (
    <video
      ref={ref}
      className="size-full object-cover"
      muted
      loop
      playsInline
      preload="metadata"
      poster={poster}
      aria-label={alt}
    >
      <source src={src} type="video/mp4" />
    </video>
  );
}
