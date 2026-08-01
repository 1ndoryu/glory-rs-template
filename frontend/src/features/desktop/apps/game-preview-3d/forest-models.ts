/* [GAME-01-VIS-3D] Compatibilidad del preview 3D.
 * Las primitivas viven en game-shared para que el fixture jugable y el boceto
 * visual compartan geometría sin importar un módulo de preview desde gameplay. */

export {
  createBroadleaf,
  createConifer,
  createFigure,
  createPond,
  createRock,
  type ForestMaterials,
} from '../game-shared/forest-models';
