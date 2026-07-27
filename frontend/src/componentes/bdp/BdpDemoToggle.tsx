/* [BDP-DEMO-TOGGLE] Botón compartido para activar/desactivar modo demo en páginas BDP. */

import { FlaskConical, Trash2 } from 'lucide-react';
import { Button } from '@/components/ui/button';

interface BdpDemoToggleProps {
  demoMode: boolean;
  onToggle: (next: boolean) => void;
}

export function BdpDemoToggle({ demoMode, onToggle }: BdpDemoToggleProps) {
  return (
    <Button
      variant="outline"
      onClick={() => onToggle(!demoMode)}
      className={demoMode ? 'bg-amber-100 hover:bg-amber-200' : ''}
      aria-pressed={demoMode}
    >
      {demoMode ? (
        <>
          <Trash2 className="size-4 mr-1.5" />
          Eliminar demo
        </>
      ) : (
        <>
          <FlaskConical className="size-4 mr-1.5" />
          Cargar demo
        </>
      )}
    </Button>
  );
}

export default BdpDemoToggle;
