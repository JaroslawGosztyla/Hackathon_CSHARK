// Palm silhouette of the Summer theme (Scene and Preview), in a 240 × 420 box: base at 60,420, crown at 150,98.
// Fronds are quadratic curves from the crown with leaflets on both sides: a sawtooth edge whose teeth lean back
// toward the crown and shrink toward the tip, so the silhouette reads as a feathery palm leaf.

interface Point {
  x: number;
  y: number;
}

const CROWN: Point = { x: 150, y: 98 };

function at(bend: Point, tip: Point, t: number): Point {
  const u = 1 - t;
  return {
    x: u * u * CROWN.x + 2 * u * t * bend.x + t * t * tip.x,
    y: u * u * CROWN.y + 2 * u * t * bend.y + t * t * tip.y,
  };
}

function normal(bend: Point, tip: Point, t: number): Point {
  const u = 1 - t;
  const dx = 2 * u * (bend.x - CROWN.x) + 2 * t * (tip.x - bend.x);
  const dy = 2 * u * (bend.y - CROWN.y) + 2 * t * (tip.y - bend.y);
  const len = Math.hypot(dx, dy) || 1;
  return { x: -dy / len, y: dx / len };
}

const f = (n: number) => n.toFixed(1);

/** One frond: `leaf` = leaflet length at the widest point (they taper to the tip). */
function frond(tip: Point, bend: Point, leaf: number): string {
  const STEPS = 14;
  const side = (sign: 1 | -1): Point[] => {
    const pts: Point[] = [];
    for (let i = 1; i < STEPS; i++) {
      const t = i / STEPS;
      const p = at(bend, tip, t);
      const n = normal(bend, tip, t);
      const size = leaf * Math.sin(Math.PI * Math.min(1, t * 1.15)) ** 0.7;
      // The rib, then a leaflet tip a little further back along the frond.
      const back = at(bend, tip, Math.max(0, t - 0.06));
      pts.push({ x: p.x + n.x * sign * 1.2, y: p.y + n.y * sign * 1.2 });
      pts.push({ x: back.x + n.x * sign * size, y: back.y + n.y * sign * size + size * 0.35 });
    }
    return pts;
  };
  const left = side(1);
  const right = side(-1).reverse();
  let d = `M${CROWN.x} ${CROWN.y}`;
  for (const p of left) d += `L${f(p.x)} ${f(p.y)}`;
  d += `L${tip.x} ${tip.y}`;
  for (const p of right) d += `L${f(p.x)} ${f(p.y)}`;
  return d + "Z";
}

export const PALM_FRONDS = [
  frond({ x: 30, y: 158 }, { x: 78, y: 66 }, 15),
  frond({ x: 48, y: 60 }, { x: 96, y: 40 }, 13),
  frond({ x: 108, y: 18 }, { x: 120, y: 44 }, 10),
  frond({ x: 186, y: 16 }, { x: 170, y: 46 }, 10),
  frond({ x: 230, y: 54 }, { x: 196, y: 42 }, 13),
  frond({ x: 240, y: 150 }, { x: 222, y: 70 }, 15),
  frond({ x: 196, y: 206 }, { x: 200, y: 118 }, 12),
  frond({ x: 96, y: 200 }, { x: 108, y: 112 }, 12),
].join("");

/** The trunk: a tapered curve, slightly thicker at the base. */
export const PALM_TRUNK = "M46 420C54 320 90 204 144 100L158 106C112 208 80 322 78 420Z";
