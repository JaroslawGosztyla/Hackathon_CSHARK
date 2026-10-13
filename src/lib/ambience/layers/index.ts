// Every ambience layer's recipe (ADR-024), by `AmbienceLayer`. Each recipe builds its sound into `kit.bus` with the
// shared noise buffers; its level at volume 1 peaks around −12 dBFS (the engine's limiter catches the sum of many).
import type { AmbienceLayer } from "$lib/types";
import type { LayerFactory } from "../kit";
import { cafe } from "./cafe";
import { fireplace } from "./fireplace";
import { forest } from "./forest";
import { ocean } from "./ocean";
import { rain } from "./rain";
import { spookyNight } from "./spookyNight";
import { thunder } from "./thunder";
import { wind } from "./wind";
import { winterCottage } from "./winterCottage";

export const LAYERS: Record<AmbienceLayer, LayerFactory> = {
  rain,
  wind,
  thunder,
  cafe,
  fireplace,
  forest,
  ocean,
  spookyNight,
  winterCottage,
};
