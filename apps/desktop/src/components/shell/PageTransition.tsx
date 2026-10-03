import type { ReactNode } from "react";
import { motion } from "motion/react";
import { useLocation } from "react-router";

/**
 * Fade + 8 px de deslocamento em 180 ms a cada troca de rota. Com `prefers-reduced-motion` o
 * `MotionConfig` do app remove o deslocamento e deixa só o fade.
 */
export function PageTransition({ children }: { children: ReactNode }) {
  const { pathname } = useLocation();

  return (
    <motion.div
      key={pathname}
      initial={{ opacity: 0, y: 8 }}
      animate={{ opacity: 1, y: 0 }}
      transition={{ duration: 0.18, ease: "easeOut" }}
      className="h-full"
    >
      {children}
    </motion.div>
  );
}
