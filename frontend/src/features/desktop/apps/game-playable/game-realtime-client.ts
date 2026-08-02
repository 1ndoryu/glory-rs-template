/* GAME-01 — Adaptador de transporte realtime del Bosque.
 * Este módulo conoce HTTP/WebSocket, pero no DOM, Three.js ni el shell. El core
 * recibe snapshots puros y el consumidor decide cómo presentarlos. */

import { generatedFetcher, unwrapGeneratedResponse, type GeneratedResponse } from '../../../../api/client';
import {
  interpolateSnapshots,
  validateGameRealtimeServerMessage,
  type GameRealtimeServerMessage,
  type GameRealtimeSnapshotPayload,
  type Vector2,
  type WorldSnapshot,
} from '../../../game-core';

const CLIENT_VERSION = 'game-01';
const HEARTBEAT_MS = 1_000;
const MOVE_SEQUENCE_START = 0;

export type GameRealtimeConnectionState = 'idle' | 'connecting' | 'connected' | 'error' | 'closed';

export interface GameRealtimeTicketResponse {
  readonly ticket: string;
}

export interface GameRealtimeSocket {
  send(data: string): void;
  close(code?: number, reason?: string): void;
  addEventListener(type: string, listener: EventListener): void;
  removeEventListener(type: string, listener: EventListener): void;
}

export interface GameRealtimeClientOptions {
  readonly ticketProvider: () => Promise<string>;
  readonly socketFactory: (url: string) => GameRealtimeSocket;
  readonly socketUrl: string;
  readonly onState?: (state: GameRealtimeConnectionState, message?: string) => void;
}

export interface GameRealtimeClientHandle {
  readonly connect: () => Promise<void>;
  readonly sendMove: (direction: Vector2) => void;
  readonly getRenderSnapshot: (nowMs?: number) => WorldSnapshot | null;
  readonly getPlayerId: () => string | null;
  readonly getMapVersion: () => string | null;
  readonly getState: () => GameRealtimeConnectionState;
  readonly destroy: () => void;
}

export async function requestGameTicket(): Promise<string> {
  const response = await generatedFetcher<GeneratedResponse<GameRealtimeTicketResponse>>(
    '/api/game/ticket',
    { method: 'POST' },
  );
  return unwrapGeneratedResponse<GameRealtimeTicketResponse>(response, [200]).ticket;
}

export function createGameRealtimeClient(
  options: GameRealtimeClientOptions,
): GameRealtimeClientHandle {
  let socket: GameRealtimeSocket | null = null;
  let state: GameRealtimeConnectionState = 'idle';
  let destroyed = false;
  let playerId: string | null = null;
  let mapVersion: string | null = null;
  let sequence = MOVE_SEQUENCE_START;
  let lastSnapshotSequence: number | null = null;
  let previousSnapshot: WorldSnapshot | null = null;
  let currentSnapshot: WorldSnapshot | null = null;
  let previousSnapshotAt = 0;
  let currentSnapshotAt = 0;
  let heartbeatTimer: ReturnType<typeof setInterval> | null = null;

  const notify = (next: GameRealtimeConnectionState, message?: string): void => {
    state = next;
    options.onState?.(next, message);
  };

  const clearHeartbeat = (): void => {
    if (heartbeatTimer === null) return;
    clearInterval(heartbeatTimer);
    heartbeatTimer = null;
  };

  const closeSocket = (code = 1000, reason = 'cliente cerrado'): void => {
    clearHeartbeat();
    if (!socket) return;
    socket.close(code, reason);
    socket = null;
  };

  const send = (message: Record<string, unknown>): boolean => {
    if (destroyed || !socket || state !== 'connected') return false;
    try {
      socket.send(JSON.stringify(message));
      return true;
    } catch {
      notify('error', 'no se pudo enviar el mensaje realtime');
      return false;
    }
  };

  const updateSnapshot = (payload: GameRealtimeSnapshotPayload): void => {
    const next: WorldSnapshot = {
      tick: payload.tick,
      entities: payload.entities.map(entity => ({ ...entity })),
    };
    const now = Date.now();
    previousSnapshot = currentSnapshot;
    previousSnapshotAt = currentSnapshotAt;
    currentSnapshot = next;
    currentSnapshotAt = now;
    lastSnapshotSequence = payload.snapshotSequence;
  };

  const handleMessage = (event: Event): void => {
    const data = (event as MessageEvent<unknown>).data;
    if (typeof data !== 'string') {
      notify('error', 'mensaje realtime no textual');
      closeSocket(1003, 'mensaje no textual');
      return;
    }
    let parsed: unknown;
    try {
      parsed = JSON.parse(data) as unknown;
    } catch {
      notify('error', 'JSON realtime inválido');
      closeSocket(1007, 'JSON inválido');
      return;
    }
    const result = validateGameRealtimeServerMessage(parsed);
    if (!result.ok) {
      notify('error', result.error);
      closeSocket(1007, 'mensaje inválido');
      return;
    }
    const message: GameRealtimeServerMessage = result.value;
    if (message.type === 'joined') {
      playerId = message.payload.playerId;
      mapVersion = message.payload.mapVersion;
      notify('connected');
      clearHeartbeat();
      heartbeatTimer = setInterval(() => {
        send({
          v: 1,
          type: 'heartbeat',
          payload: { lastSnapshotSequence: lastSnapshotSequence ?? 0 },
        });
      }, HEARTBEAT_MS);
      return;
    }
    if (message.type === 'snapshot') {
      if (lastSnapshotSequence !== null && message.payload.snapshotSequence <= lastSnapshotSequence) return;
      updateSnapshot(message.payload);
      return;
    }
    if (message.type === 'error') {
      if (message.payload.fatal) {
        notify('error', message.payload.message);
        closeSocket(1008, message.payload.code);
      } else if (state === 'connected') {
        options.onState?.('connected', message.payload.message);
      }
    }
  };

  const handleOpen = (): void => {
    if (destroyed || !socket) return;
    const ticketPromise = options.ticketProvider();
    void ticketPromise.then((ticket) => {
      if (destroyed || !socket) return;
      const join = {
        v: 1,
        type: 'join',
        payload: { ticket, clientVersion: CLIENT_VERSION },
      };
      try {
        socket.send(JSON.stringify(join));
      } catch {
        notify('error', 'no se pudo enviar el join realtime');
        closeSocket(1011, 'join fallido');
      }
    }).catch((error: unknown) => {
      notify('error', error instanceof Error ? error.message : 'no se pudo obtener ticket');
      closeSocket(1008, 'ticket inválido');
    });


  };

  const handleError = (): void => {
    if (!destroyed) notify('error', 'conexión realtime no disponible');
  };

  const handleClose = (): void => {
    clearHeartbeat();
    socket = null;
    if (!destroyed && state !== 'error') notify('closed');
  };

  const connect = async (): Promise<void> => {
    if (destroyed || state === 'connecting' || state === 'connected') return;
    notify('connecting');
    try {
      socket = options.socketFactory(options.socketUrl);
      socket.addEventListener('open', handleOpen);
      socket.addEventListener('message', handleMessage);
      socket.addEventListener('error', handleError);
      socket.addEventListener('close', handleClose);
    } catch (error: unknown) {
      notify('error', error instanceof Error ? error.message : 'no se pudo abrir realtime');
    }
  };

  return {
    connect,
    sendMove: (direction: Vector2): void => {
      if (state !== 'connected') return;
      send({
        v: 1,
        type: 'move',
        payload: { sequence: sequence++, direction },
      });
    },
    getRenderSnapshot: (nowMs = Date.now()): WorldSnapshot | null => {
      if (!currentSnapshot) return null;
      if (!previousSnapshot || currentSnapshotAt <= previousSnapshotAt) return currentSnapshot;
      const alpha = (nowMs - currentSnapshotAt) / (currentSnapshotAt - previousSnapshotAt) + 1;
      return interpolateSnapshots(previousSnapshot, currentSnapshot, alpha);
    },
    getPlayerId: () => playerId,
    getMapVersion: () => mapVersion,
    getState: () => state,
    destroy: (): void => {
      if (destroyed) return;
      destroyed = true;
      clearHeartbeat();
      if (socket) {
        socket.removeEventListener('open', handleOpen);
        socket.removeEventListener('message', handleMessage);
        socket.removeEventListener('error', handleError);
        socket.removeEventListener('close', handleClose);
      }
      closeSocket(1000, 'vista destruida');
      notify('closed');
    },
  };
}

export function defaultGameSocketUrl(locationLike: Location = window.location): string {
  const protocol = locationLike.protocol === 'https:' ? 'wss:' : 'ws:';
  return `${protocol}//${locationLike.host}/api/game/ws`;
}

