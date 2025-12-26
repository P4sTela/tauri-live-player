import { useState, useEffect } from "react";
import {
  Plus,
  Trash2,
  Edit2,
  Monitor,
  Radio,
  Volume2,
  Square,
  Maximize,
  AppWindow,
  ChevronDown,
} from "lucide-react";
import { useProjectStore } from "../../stores/projectStore";
import { useOutputStore } from "../../stores/outputStore";
import { Button } from "../ui/button";
import { Input } from "../ui/input";
import { Label } from "../ui/label";
import { Switch } from "../ui/switch";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "../ui/select";
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
  DialogFooter,
} from "../ui/dialog";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from "../ui/dropdown-menu";
import { Card, CardContent } from "../ui/card";
import { generateId } from "../../lib/utils";
import { cn } from "../../lib/utils";
import type { OutputTarget, OutputType, AudioDriver } from "../../types";

const OUTPUT_TYPE_ICONS = {
  display: Monitor,
  ndi: Radio,
  audio: Volume2,
};

const OUTPUT_TYPE_LABELS = {
  display: "Display",
  ndi: "NDI",
  audio: "Audio",
};

interface OutputManagerProps {
  compact?: boolean;
}

export function OutputManager({ compact = false }: OutputManagerProps) {
  const { project, addOutput, updateOutput, removeOutput } = useProjectStore();
  const {
    monitors,
    isLoadingMonitors,
    fetchMonitors,
    openOutput,
    closeOutput,
    isOutputOpen,
  } = useOutputStore();

  const [editingOutput, setEditingOutput] = useState<OutputTarget | null>(null);
  const [isAdding, setIsAdding] = useState(false);

  // モニター一覧を取得
  useEffect(() => {
    fetchMonitors();
  }, [fetchMonitors]);

  if (!project) {
    return null;
  }

  const handleAddOutput = (type: OutputType) => {
    const count = project.outputs.filter((o) => o.type === type).length + 1;
    const newOutput: OutputTarget = {
      id: generateId(),
      name: `${OUTPUT_TYPE_LABELS[type]} ${count}`,
      type,
      brightness: null, // Master連動
      ...(type === "display" && { displayIndex: 0, fullscreen: true }),
      ...(type === "ndi" && { ndiName: `LivePlayer_${count}` }),
      ...(type === "audio" && { audioDriver: "auto" as AudioDriver }),
    };
    addOutput(newOutput);
    setIsAdding(false);
  };

  const handleSaveOutput = () => {
    if (editingOutput) {
      updateOutput(editingOutput.id, editingOutput);
      setEditingOutput(null);
    }
  };

  const handleDeleteOutput = (id: string) => {
    // 開いていたら閉じる
    if (isOutputOpen(id)) {
      closeOutput(id);
    }
    removeOutput(id);
  };

  const handleCloseOutput = async (output: OutputTarget) => {
    await closeOutput(output.id);
  };

  const handleOpenFullscreen = async (
    output: OutputTarget,
    monitorIndex: number,
  ) => {
    const monitor = monitors.find((m) => m.index === monitorIndex);
    if (monitor) {
      await openOutput(output, monitor);
    }
  };

  const handleOpenWindowed = async (output: OutputTarget) => {
    await openOutput(output, null);
  };

  if (compact) {
    return (
      <div className="flex flex-wrap gap-2">
        {project.outputs.map((output) => {
          const Icon = OUTPUT_TYPE_ICONS[output.type];
          const isOpen = isOutputOpen(output.id);
          return (
            <div
              key={output.id}
              className={cn(
                "flex items-center gap-1 px-2 py-1 rounded text-sm cursor-pointer transition-colors",
                isOpen ? "bg-green-500/20 text-green-500" : "bg-muted",
              )}
              onClick={() =>
                isOpen ? handleCloseOutput(output) : handleOpenWindowed(output)
              }
            >
              <Icon className="w-3 h-3" />
              <span>{output.name}</span>
              {isOpen && <span className="w-2 h-2 rounded-full bg-green-500" />}
            </div>
          );
        })}
        <Button
          variant="ghost"
          size="sm"
          className="h-7"
          onClick={() => setIsAdding(true)}
        >
          <Plus className="w-3 h-3" />
        </Button>
      </div>
    );
  }

  return (
    <div className="space-y-4">
      {/* Output List */}
      {project.outputs.length === 0 ? (
        <div className="text-center text-muted-foreground py-8">
          <p className="mb-4">No outputs configured.</p>
          <Button variant="outline" onClick={() => setIsAdding(true)}>
            <Plus className="w-4 h-4 mr-2" />
            Add Output
          </Button>
        </div>
      ) : (
        <div className="space-y-2">
          {project.outputs.map((output) => {
            const Icon = OUTPUT_TYPE_ICONS[output.type];
            const isOpen = isOutputOpen(output.id);
            const monitor = monitors.find(
              (m) => m.index === (output.displayIndex ?? 0),
            );

            return (
              <Card
                key={output.id}
                className={cn(isOpen && "border-green-500/50")}
              >
                <CardContent className="p-3">
                  <div className="flex items-center gap-3">
                    {/* Icon with status */}
                    <div
                      className={cn(
                        "w-10 h-10 rounded flex items-center justify-center relative",
                        output.type === "display" &&
                          "bg-blue-500/20 text-blue-500",
                        output.type === "ndi" &&
                          "bg-purple-500/20 text-purple-500",
                        output.type === "audio" &&
                          "bg-green-500/20 text-green-500",
                      )}
                    >
                      <Icon className="w-5 h-5" />
                      {isOpen && (
                        <span className="absolute -top-1 -right-1 w-3 h-3 rounded-full bg-green-500 border-2 border-background" />
                      )}
                    </div>

                    {/* Info */}
                    <div className="flex-1 min-w-0">
                      <div className="font-medium">{output.name}</div>
                      <div className="text-xs text-muted-foreground">
                        {output.type === "display" && monitor && (
                          <>
                            {monitor.name || `Display ${monitor.index}`} (
                            {monitor.width}x{monitor.height})
                            {monitor.isPrimary && " - Primary"}
                          </>
                        )}
                        {output.type === "display" && !monitor && (
                          <>Display {output.displayIndex ?? 0}</>
                        )}
                        {output.type === "ndi" && output.ndiName}
                        {output.type === "audio" &&
                          `${output.audioDriver || "auto"}`}
                      </div>
                    </div>

                    {/* Open/Close for Display outputs */}
                    {output.type === "display" &&
                      (isOpen ? (
                        <Button
                          variant="default"
                          size="sm"
                          onClick={() => handleCloseOutput(output)}
                          className="gap-1 bg-green-600 hover:bg-green-700"
                        >
                          <Square className="w-3 h-3" />
                          Close
                        </Button>
                      ) : (
                        <DropdownMenu>
                          <DropdownMenuTrigger asChild>
                            <Button
                              variant="outline"
                              size="sm"
                              className="gap-1"
                            >
                              Open
                              <ChevronDown className="w-3 h-3" />
                            </Button>
                          </DropdownMenuTrigger>
                          <DropdownMenuContent align="end">
                            <DropdownMenuItem
                              onClick={() => handleOpenWindowed(output)}
                            >
                              <AppWindow className="w-4 h-4 mr-2" />
                              Window
                            </DropdownMenuItem>
                            <DropdownMenuSeparator />
                            {monitors.map((m) => (
                              <DropdownMenuItem
                                key={m.index}
                                onClick={() =>
                                  handleOpenFullscreen(output, m.index)
                                }
                              >
                                <Maximize className="w-4 h-4 mr-2" />
                                {m.name || `Display ${m.index}`}
                                {m.isPrimary && " ★"}
                              </DropdownMenuItem>
                            ))}
                          </DropdownMenuContent>
                        </DropdownMenu>
                      ))}

                    {/* Edit */}
                    <Button
                      variant="ghost"
                      size="icon"
                      onClick={() => setEditingOutput({ ...output })}
                    >
                      <Edit2 className="w-4 h-4" />
                    </Button>

                    {/* Delete */}
                    <Button
                      variant="ghost"
                      size="icon"
                      className="text-destructive"
                      onClick={() => handleDeleteOutput(output.id)}
                    >
                      <Trash2 className="w-4 h-4" />
                    </Button>
                  </div>
                </CardContent>
              </Card>
            );
          })}

          <Button
            variant="outline"
            className="w-full"
            onClick={() => setIsAdding(true)}
          >
            <Plus className="w-4 h-4 mr-2" />
            Add Output
          </Button>
        </div>
      )}

      {/* Add Output Dialog */}
      <Dialog open={isAdding} onOpenChange={setIsAdding}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>Add Output</DialogTitle>
          </DialogHeader>
          <div className="grid grid-cols-3 gap-4">
            <Button
              variant="outline"
              className="h-24 flex-col gap-2"
              onClick={() => handleAddOutput("display")}
            >
              <Monitor className="w-8 h-8 text-blue-500" />
              <span>Display</span>
            </Button>
            <Button
              variant="outline"
              className="h-24 flex-col gap-2"
              onClick={() => handleAddOutput("ndi")}
            >
              <Radio className="w-8 h-8 text-purple-500" />
              <span>NDI</span>
            </Button>
            <Button
              variant="outline"
              className="h-24 flex-col gap-2"
              onClick={() => handleAddOutput("audio")}
            >
              <Volume2 className="w-8 h-8 text-green-500" />
              <span>Audio</span>
            </Button>
          </div>
        </DialogContent>
      </Dialog>

      {/* Edit Output Dialog */}
      <Dialog
        open={!!editingOutput}
        onOpenChange={() => setEditingOutput(null)}
      >
        <DialogContent>
          <DialogHeader>
            <DialogTitle>Edit Output</DialogTitle>
          </DialogHeader>
          {editingOutput && (
            <div className="space-y-4">
              <div>
                <Label htmlFor="output-name">Name</Label>
                <Input
                  id="output-name"
                  value={editingOutput.name}
                  onChange={(e) =>
                    setEditingOutput({ ...editingOutput, name: e.target.value })
                  }
                />
              </div>

              {editingOutput.type === "display" && (
                <>
                  <div>
                    <Label htmlFor="display-select">Monitor</Label>
                    <Select
                      value={String(editingOutput.displayIndex ?? 0)}
                      onValueChange={(value) =>
                        setEditingOutput({
                          ...editingOutput,
                          displayIndex: parseInt(value),
                        })
                      }
                    >
                      <SelectTrigger>
                        <SelectValue placeholder="Select monitor" />
                      </SelectTrigger>
                      <SelectContent>
                        {isLoadingMonitors ? (
                          <SelectItem value="0" disabled>
                            Loading...
                          </SelectItem>
                        ) : monitors.length === 0 ? (
                          <SelectItem value="0">Display 0 (Default)</SelectItem>
                        ) : (
                          monitors.map((m) => (
                            <SelectItem key={m.index} value={String(m.index)}>
                              {m.name || `Display ${m.index}`} ({m.width}x
                              {m.height}){m.isPrimary && " ★"}
                            </SelectItem>
                          ))
                        )}
                      </SelectContent>
                    </Select>
                  </div>
                  <div className="flex items-center justify-between">
                    <Label htmlFor="fullscreen">Fullscreen</Label>
                    <Switch
                      id="fullscreen"
                      checked={editingOutput.fullscreen ?? true}
                      onCheckedChange={(checked: boolean) =>
                        setEditingOutput({
                          ...editingOutput,
                          fullscreen: checked,
                        })
                      }
                    />
                  </div>
                </>
              )}

              {editingOutput.type === "ndi" && (
                <div>
                  <Label htmlFor="ndi-name">NDI Name</Label>
                  <Input
                    id="ndi-name"
                    value={editingOutput.ndiName ?? ""}
                    onChange={(e) =>
                      setEditingOutput({
                        ...editingOutput,
                        ndiName: e.target.value,
                      })
                    }
                  />
                  <p className="text-xs text-muted-foreground mt-1">
                    This name will be visible on the network
                  </p>
                </div>
              )}

              {editingOutput.type === "audio" && (
                <div>
                  <Label htmlFor="audio-driver">Audio Driver</Label>
                  <Select
                    value={editingOutput.audioDriver ?? "auto"}
                    onValueChange={(value) =>
                      setEditingOutput({
                        ...editingOutput,
                        audioDriver: value as AudioDriver,
                      })
                    }
                  >
                    <SelectTrigger>
                      <SelectValue />
                    </SelectTrigger>
                    <SelectContent>
                      <SelectItem value="auto">Auto</SelectItem>
                      <SelectItem value="asio">ASIO (Windows)</SelectItem>
                      <SelectItem value="wasapi">WASAPI (Windows)</SelectItem>
                      <SelectItem value="coreaudio">
                        Core Audio (macOS)
                      </SelectItem>
                      <SelectItem value="jack">JACK (Linux)</SelectItem>
                      <SelectItem value="alsa">ALSA (Linux)</SelectItem>
                    </SelectContent>
                  </Select>
                </div>
              )}
            </div>
          )}
          <DialogFooter>
            <Button variant="outline" onClick={() => setEditingOutput(null)}>
              Cancel
            </Button>
            <Button onClick={handleSaveOutput}>Save</Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </div>
  );
}
