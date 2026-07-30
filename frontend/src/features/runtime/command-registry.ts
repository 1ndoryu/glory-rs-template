/* wandori.us — Command Registry
 * Registro central de comandos del OS.
 * Comandos = acciones invocables desde menús, clic derecho, atajos o taskbar.
 * Cada comando tiene un id, label opcional y función execute. */

export interface Command {
  /** Identificador único (ej: 'app:open', 'window:close'). */
  readonly id: string;
  /** Etiqueta visible para menús. */
  readonly label?: string;
  /** Atajo de teclado opcional. */
  readonly shortcut?: string;
  /** Función a ejecutar. */
  readonly execute: (args?: Record<string, unknown>) => void | Promise<void>;
}

class CommandRegistryClass {
  private commands = new Map<string, Command>();

  /** Registrar un comando. */
  register(command: Command): void {
    this.commands.set(command.id, command);
  }

  /** Registrar múltiples comandos. */
  registerAll(commands: Command[]): void {
    for (const cmd of commands) this.register(cmd);
  }

  /** Ejecutar un comando por ID. */
  async execute(id: string, args?: Record<string, unknown>): Promise<boolean> {
    const cmd = this.commands.get(id);
    if (!cmd) return false;
    await cmd.execute(args);
    return true;
  }

  /** Obtener un comando por ID. */
  get(id: string): Command | undefined {
    return this.commands.get(id);
  }

  /** Listar todos los comandos registrados. */
  getAll(): readonly Command[] {
    return Array.from(this.commands.values());
  }

  /** Listar comandos con shortcut para el handler de teclado. */
  getWithShortcuts(): readonly Command[] {
    return this.getAll().filter(c => c.shortcut);
  }
}

/** Instancia singleton del registry. */
export const CommandRegistry = new CommandRegistryClass();
