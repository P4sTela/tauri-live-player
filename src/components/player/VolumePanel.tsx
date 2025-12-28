import { Volume2 } from "lucide-react";
import { useProjectStore } from "../../stores/projectStore";
import { Slider } from "../ui/slider";

interface VolumePanelProps {
  compact?: boolean;
}

export function VolumePanel({ compact = false }: VolumePanelProps) {
  const { project, setMasterVolume } = useProjectStore();

  if (!project) {
    return null;
  }

  const handleChange = (value: number[]) => {
    setMasterVolume(value[0]);
  };

  // Compact mode - just slider inline
  if (compact) {
    return (
      <div className="flex items-center gap-2">
        <Volume2 className="w-4 h-4 text-muted-foreground" />
        <Slider
          value={[project.masterVolume]}
          min={0}
          max={100}
          step={1}
          onValueChange={handleChange}
          className="w-24"
        />
        <span className="text-sm text-muted-foreground w-8">
          {Math.round(project.masterVolume)}%
        </span>
      </div>
    );
  }

  // Full mode
  return (
    <div className="space-y-2">
      <div className="flex items-center gap-2">
        <Volume2 className="w-4 h-4" />
        <span className="text-sm font-medium">Master Volume</span>
        <span className="ml-auto text-sm text-muted-foreground">
          {Math.round(project.masterVolume)}%
        </span>
      </div>
      <Slider
        value={[project.masterVolume]}
        min={0}
        max={100}
        step={1}
        onValueChange={handleChange}
      />
    </div>
  );
}
