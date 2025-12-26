import { Sun, Link2, Link2Off } from "lucide-react";
import { useProjectStore } from "../../stores/projectStore";
import { Slider } from "../ui/slider";
import { Button } from "../ui/button";
import { invoke } from "@tauri-apps/api/core";

interface BrightnessPanelProps {
  compact?: boolean;
}

export function BrightnessPanel({ compact = false }: BrightnessPanelProps) {
  const { project, setMasterBrightness, updateOutput } = useProjectStore();

  if (!project) {
    return null;
  }

  const handleMasterChange = (value: number[]) => {
    setMasterBrightness(value[0]);
  };

  const handleOutputChange = async (outputId: string, value: number[]) => {
    const val = value[0];
    try {
      await invoke("set_output_brightness", { outputId, value: val });
      updateOutput(outputId, { brightness: val });
    } catch (e) {
      console.error("Failed to set output brightness:", e);
    }
  };

  const toggleLink = async (
    outputId: string,
    currentBrightness: number | null | undefined,
  ) => {
    const isLinked = currentBrightness == null;

    if (isLinked) {
      // Unlink: 現在のMaster値を個別値として設定
      try {
        await invoke("set_output_brightness", {
          outputId,
          value: project.masterBrightness,
        });
        updateOutput(outputId, { brightness: project.masterBrightness });
      } catch (e) {
        console.error("Failed to unlink output:", e);
      }
    } else {
      // Link: Master連動に戻す
      try {
        await invoke("set_output_brightness", { outputId, value: null });
        updateOutput(outputId, { brightness: null });
      } catch (e) {
        console.error("Failed to link output:", e);
      }
    }
  };

  const videoOutputs = project.outputs.filter((o) => o.type !== "audio");

  // Compact mode - just master slider inline
  if (compact) {
    return (
      <div className="flex items-center gap-2">
        <Sun className="w-4 h-4 text-muted-foreground" />
        <Slider
          value={[project.masterBrightness]}
          min={0}
          max={100}
          step={1}
          onValueChange={handleMasterChange}
          className="w-24"
        />
        <span className="text-sm text-muted-foreground w-8">
          {Math.round(project.masterBrightness)}%
        </span>
      </div>
    );
  }

  // Full mode
  return (
    <div className="space-y-4">
      {/* Master Brightness */}
      <div className="space-y-2">
        <div className="flex items-center gap-2">
          <Sun className="w-4 h-4" />
          <span className="text-sm font-medium">Master</span>
          <span className="ml-auto text-sm text-muted-foreground">
            {Math.round(project.masterBrightness)}%
          </span>
        </div>
        <Slider
          value={[project.masterBrightness]}
          min={0}
          max={100}
          step={1}
          onValueChange={handleMasterChange}
        />
      </div>

      {/* Output Brightness */}
      {videoOutputs.length > 0 && (
        <div className="space-y-3 pt-2 border-t">
          <span className="text-xs text-muted-foreground uppercase tracking-wide">
            Outputs
          </span>
          {videoOutputs.map((output) => {
            const isLinked = output.brightness == null;
            const value = output.brightness ?? project.masterBrightness;

            return (
              <div key={output.id} className="space-y-2">
                <div className="flex items-center gap-2">
                  <span className="text-sm">{output.name}</span>
                  <Button
                    variant="ghost"
                    size="icon"
                    className="h-6 w-6 ml-auto"
                    onClick={() => toggleLink(output.id, output.brightness)}
                    title={isLinked ? "Unlink from Master" : "Link to Master"}
                  >
                    {isLinked ? (
                      <Link2 className="w-3 h-3" />
                    ) : (
                      <Link2Off className="w-3 h-3" />
                    )}
                  </Button>
                  <span className="text-sm text-muted-foreground w-10 text-right">
                    {Math.round(value)}%
                  </span>
                </div>
                <Slider
                  value={[value]}
                  min={0}
                  max={100}
                  step={1}
                  onValueChange={(v) => handleOutputChange(output.id, v)}
                  disabled={isLinked}
                />
              </div>
            );
          })}
        </div>
      )}

      {videoOutputs.length === 0 && (
        <p className="text-sm text-muted-foreground pt-2 border-t">
          No video outputs configured.
        </p>
      )}
    </div>
  );
}
