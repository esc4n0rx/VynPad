"use client";

import { motion } from "motion/react";
import type { ReactNode } from "react";

interface RevealProps {
  children: ReactNode;
  delay?: number;
  className?: string;
}

/**
 * Entra com fade e um deslocamento curto quando aparece na tela, uma vez só. Para quem pediu menos
 * movimento, o `MotionConfig reducedMotion="user"` (em `providers.tsx`) tira o deslocamento e
 * deixa só o fade. O conteúdo já está no HTML (nada de esconder do buscador); só a
 * apresentação anima.
 */
export function Reveal({ children, delay = 0, className }: RevealProps) {
  return (
    <motion.div
      className={`reveal ${className ?? ""}`}
      initial={{ opacity: 0, y: 16 }}
      whileInView={{ opacity: 1, y: 0 }}
      viewport={{ once: true, margin: "0px 0px -80px 0px" }}
      transition={{ duration: 0.65, delay, ease: [0.22, 1, 0.36, 1] }}
    >
      {children}
    </motion.div>
  );
}
