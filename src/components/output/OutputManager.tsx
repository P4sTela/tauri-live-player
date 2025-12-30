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
  GripVertical,
  Share2,
  Layers,
} from "lucide-react";
import {
  DndContext,
  closestCenter,
  KeyboardSensor,
  PointerSensor,
  useSensor,
  useSensors,
  DragEndEvent,
} from "@dnd-kit/core";
import {
  SortableContext,
  sortableKeyboardCoordinates,
  useSortable,
  verticalListSortingStrategy,
} from "@dnd-kit/sortable";
import { CSS } from "@dnd-kit/utilities";
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
import { generateId } from "../../lib/utils";
import { cn } from "../../lib/utils";
import type {
  OutputTarget,
  OutputType,
  AudioDriver,
  MonitorInfo,
} from "../../types";

const OUTPUT_TYPE_ICONS: Record<OutputType, typeof Monitor> = {
  display: Monitor,
  ndi: Radio,
  audio: Volume2,
  syphon: Share2,
  spout: Layers,
};

const OUTPUT_TYPE_LABELS: Record<OutputType, string> = {
  display: "Display",
  ndi: "NDI",
  audio: "Audio",
  syphon: "Syphon",
  spout: "Spout",
};

// Sortable output item component
interface SortableOutputItemProps {
  output: OutputTarget;
  isOpen: boolean;
  monitor: MonitorInfo | undefined;
  monitors: MonitorInfo[];
  onEdit: () => void;
  onDelete: () => void;
  onClose: () => void;
  onOpenWindowed: () => void;
  onOpenFullscreen: (monitorIndex: number) => void;
}

function SortableOutputItem({
  output,
  isOpen,
  monitor,
  monitors,
  onEdit,
  onDelete,
  onClose,
  onOpenWindowed,
  onOpenFullscreen,
}: SortableOutputItemProps) {
  const {
    attributes,
    listeners,
    setNodeRef,
    transform,
    transition,
    isDragging,
  } = useSortable({ id: output.id });

  const style = {
    transform: CSS.Transform.toString(transform),
    transition,
  };

  const Icon = OUTPUT_TYPE_ICONS[output.type];

  return (
    <div
      ref={setNodeRef}
      style={style}
      className={cn(
        "flex items-center gap-2 px-2 py-1.5 rounded-md hover:bg-muted/50 group",
        isOpen && "bg-green-500/5",
        isDragging && "opacity-50 bg-muted",
      )}
    >
      {/* Drag handle */}
      <div
        {...attributes}
        {...listeners}
        className="cursor-grab active:cursor-grabbing"
      >
        <GripVertical className="w-3 h-3 text-muted-foreground" />
      </div>

      {/* Icon */}
      <div
        className={cn(
          "w-6 h-6 rounded flex items-center justify-center shrink-0",
          output.type === "display" && "bg-blue-500/20 text-blue-500",
          output.type === "ndi" && "bg-purple-500/20 text-purple-500",
          output.type === "audio" && "bg-green-500/20 text-green-500",
          output.type === "syphon" && "bg-orange-500/20 text-orange-500",
          output.type === "spout" && "bg-cyan-500/20 text-cyan-500",
        )}
      >
        <Icon className="w-3.5 h-3.5" />
      </div>

      {/* Info */}
      <div className="flex-1 min-w-0">
        <div className="text-sm font-medium leading-tight flex items-center gap-1.5">
          {output.name}
          {isOpen && <span className="w-1.5 h-1.5 rounded-full bg-green-500" />}
        </div>
        <div className="text-xs text-muted-foreground truncate">
          {output.type === "display" && monitor && (
            <>{monitor.name || `Display ${monitor.index}`}</>
          )}
          {output.type === "display" && !monitor && (
            <>Display {output.displayIndex ?? 0}</>
          )}
          {output.type === "ndi" && output.ndiName}
          {output.type === "syphon" && (output.syphonName || "TauriLivePlayer")}
          {output.type === "spout" && (output.spoutName || "TauriLivePlayer")}
          {output.type === "audio" && (output.audioDriver || "auto")}
        </div>
      </div>

      {/* Open/Close for Display */}
      {output.type === "display" &&
        (isOpen ? (
          <Button
            variant="ghost"
            size="sm"
            onClick={onClose}
            className="h-6 px-2 text-xs bg-green-600 hover:bg-green-700 text-white"
          >
            <Square className="w-3 h-3 mr-1" />
            Close
          </Button>
        ) : (
          <DropdownMenu>
            <DropdownMenuTrigger asChild>
              <Button variant="outline" size="sm" className="h-6 px-2 text-xs">
                Open
                <ChevronDown className="w-3 h-3 ml-1" />
              </Button>
            </DropdownMenuTrigger>
            <DropdownMenuContent align="end">
              <DropdownMenuItem onClick={onOpenWindowed}>
                <AppWindow className="w-3.5 h-3.5 mr-2" />
                Window
              </DropdownMenuItem>
              <DropdownMenuSeparator />
              {monitors.map((m) => (
                <DropdownMenuItem
                  key={m.index}
                  onClick={() => onOpenFullscreen(m.index)}
                >
                  <Maximize className="w-3.5 h-3.5 mr-2" />
                  {m.name || `Display ${m.index}`}
                  {m.isPrimary && " ★"}
                </DropdownMenuItem>
              ))}
            </DropdownMenuContent>
          </DropdownMenu>
        ))}

      {/* NDI/Syphon/Spout indicator - auto-enabled when cue plays */}
      {(output.type === "ndi" ||
        output.type === "syphon" ||
        output.type === "spout") && (
        <span className="text-xs text-muted-foreground px-2">Auto</span>
      )}

      {/* Actions */}
      <div className="flex items-center opacity-0 group-hover:opacity-100">
        <Button
          variant="ghost"
          size="icon"
          className="h-6 w-6"
          onClick={onEdit}
        >
          <Edit2 className="w-3 h-3" />
        </Button>
        <Button
          variant="ghost"
          size="icon"
          className="h-6 w-6 text-destructive hover:text-destructive"
          onClick={onDelete}
        >
          <Trash2 className="w-3 h-3" />
        </Button>
      </div>
    </div>
  );
}

interface OutputManagerProps {
  compact?: boolean;
}

export function OutputManager({ compact = false }: OutputManagerProps) {
  const { project, addOutput, updateOutput, removeOutput, reorderOutputs } =
    useProjectStore();
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

  const sensors = useSensors(
    useSensor(PointerSensor),
    useSensor(KeyboardSensor, {
      coordinateGetter: sortableKeyboardCoordinates,
    }),
  );

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
      brightness: null,
      ...(type === "display" && { displayIndex: 0, fullscreen: true }),
      ...(type === "ndi" && { ndiName: `LivePlayer_${count}` }),
      ...(type === "syphon" && { syphonName: `LivePlayer_${count}` }),
      ...(type === "spout" && { spoutName: `LivePlayer_${count}` }),
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

  const handleDragEnd = (event: DragEndEvent) => {
    const { active, over } = event;
    if (!over || active.id === over.id) return;

    const oldIndex = project.outputs.findIndex((o) => o.id === active.id);
    const newIndex = project.outputs.findIndex((o) => o.id === over.id);

    if (oldIndex !== -1 && newIndex !== -1) {
      reorderOutputs(oldIndex, newIndex);
    }
  };

  if (compact) {
    return (
      <div className="flex flex-wrap gap-1.5">
        {project.outputs.map((output) => {
          const Icon = OUTPUT_TYPE_ICONS[output.type];
          const isOpen = isOutputOpen(output.id);
          return (
            <div
              key={output.id}
              className={cn(
                "flex items-center gap-1 px-2 py-0.5 rounded text-xs cursor-pointer transition-colors",
                isOpen ? "bg-green-500/20 text-green-500" : "bg-muted",
              )}
              onClick={() =>
                isOpen ? handleCloseOutput(output) : handleOpenWindowed(output)
              }
            >
              <Icon className="w-3 h-3" />
              <span>{output.name}</span>
              {isOpen && (
                <span className="w-1.5 h-1.5 rounded-full bg-green-500" />
              )}
            </div>
          );
        })}
        <Button
          variant="ghost"
          size="sm"
          className="h-5 px-1.5"
          onClick={() => setIsAdding(true)}
        >
          <Plus className="w-3 h-3" />
        </Button>
      </div>
    );
  }

  return (
    <div className="space-y-1">
      {/* Output List */}
      {project.outputs.length === 0 ? (
        <div className="text-center text-muted-foreground py-4 text-sm">
          <p className="mb-2">No outputs configured.</p>
          <Button variant="outline" size="sm" onClick={() => setIsAdding(true)}>
            <Plus className="w-3 h-3 mr-1" />
            Add Output
          </Button>
        </div>
      ) : (
        <DndContext
          sensors={sensors}
          collisionDetection={closestCenter}
          onDragEnd={handleDragEnd}
        >
          <SortableContext
            items={project.outputs.map((o) => o.id)}
            strategy={verticalListSortingStrategy}
          >
            <div className="space-y-0.5">
              {project.outputs.map((output) => {
                const monitor = monitors.find(
                  (m) => m.index === (output.displayIndex ?? 0),
                );

                return (
                  <SortableOutputItem
                    key={output.id}
                    output={output}
                    isOpen={isOutputOpen(output.id)}
                    monitor={monitor}
                    monitors={monitors}
                    onEdit={() => setEditingOutput({ ...output })}
                    onDelete={() => handleDeleteOutput(output.id)}
                    onClose={() => handleCloseOutput(output)}
                    onOpenWindowed={() => handleOpenWindowed(output)}
                    onOpenFullscreen={(idx) =>
                      handleOpenFullscreen(output, idx)
                    }
                  />
                );
              })}

              <Button
                variant="ghost"
                size="sm"
                className="w-full h-7 text-xs text-muted-foreground"
                onClick={() => setIsAdding(true)}
              >
                <Plus className="w-3 h-3 mr-1" />
                Add Output
              </Button>
            </div>
          </SortableContext>
        </DndContext>
      )}

      {/* Add Output Dialog */}
      <Dialog open={isAdding} onOpenChange={setIsAdding}>
        <DialogContent className="sm:max-w-sm">
          <DialogHeader>
            <DialogTitle>Add Output</DialogTitle>
          </DialogHeader>
          <div className="grid grid-cols-3 gap-2">
            <Button
              variant="outline"
              className="h-16 flex-col gap-1 text-xs"
              onClick={() => handleAddOutput("display")}
            >
              <Monitor className="w-5 h-5 text-blue-500" />
              Display
            </Button>
            <Button
              variant="outline"
              className="h-16 flex-col gap-1 text-xs"
              onClick={() => handleAddOutput("ndi")}
            >
              <Radio className="w-5 h-5 text-purple-500" />
              NDI
            </Button>
            <Button
              variant="outline"
              className="h-16 flex-col gap-1 text-xs"
              onClick={() => handleAddOutput("audio")}
            >
              <Volume2 className="w-5 h-5 text-green-500" />
              Audio
            </Button>
            <Button
              variant="outline"
              className="h-16 flex-col gap-1 text-xs"
              onClick={() => handleAddOutput("syphon")}
            >
              <Share2 className="w-5 h-5 text-orange-500" />
              Syphon
            </Button>
            <Button
              variant="outline"
              className="h-16 flex-col gap-1 text-xs"
              onClick={() => handleAddOutput("spout")}
            >
              <Layers className="w-5 h-5 text-cyan-500" />
              Spout
            </Button>
          </div>
        </DialogContent>
      </Dialog>

      {/* Edit Output Dialog */}
      <Dialog
        open={!!editingOutput}
        onOpenChange={() => setEditingOutput(null)}
      >
        <DialogContent className="sm:max-w-sm">
          <DialogHeader>
            <DialogTitle>Edit Output</DialogTitle>
          </DialogHeader>
          {editingOutput && (
            <div className="space-y-3">
              <div className="space-y-1">
                <Label htmlFor="output-name" className="text-xs">
                  Name
                </Label>
                <Input
                  id="output-name"
                  value={editingOutput.name}
                  onChange={(e) =>
                    setEditingOutput({ ...editingOutput, name: e.target.value })
                  }
                  className="h-8"
                />
              </div>

              <div className="space-y-1">
                <Label htmlFor="output-type" className="text-xs">
                  Type
                </Label>
                <Select
                  value={editingOutput.type}
                  onValueChange={(value: OutputType) =>
                    setEditingOutput({
                      ...editingOutput,
                      type: value,
                      // Set default values for the new type
                      ...(value === "display" && {
                        displayIndex: editingOutput.displayIndex ?? 0,
                        fullscreen: editingOutput.fullscreen ?? true,
                      }),
                      ...(value === "ndi" && {
                        ndiName: editingOutput.ndiName ?? "LivePlayer",
                      }),
                      ...(value === "syphon" && {
                        syphonName: editingOutput.syphonName ?? "LivePlayer",
                      }),
                      ...(value === "spout" && {
                        spoutName: editingOutput.spoutName ?? "LivePlayer",
                      }),
                      ...(value === "audio" && {
                        audioDriver: editingOutput.audioDriver ?? "auto",
                      }),
                    })
                  }
                >
                  <SelectTrigger className="h-8">
                    <SelectValue />
                  </SelectTrigger>
                  <SelectContent>
                    <SelectItem value="display">
                      <div className="flex items-center gap-2">
                        <Monitor className="w-4 h-4 text-blue-500" />
                        Display
                      </div>
                    </SelectItem>
                    <SelectItem value="ndi">
                      <div className="flex items-center gap-2">
                        <Radio className="w-4 h-4 text-purple-500" />
                        NDI
                      </div>
                    </SelectItem>
                    <SelectItem value="audio">
                      <div className="flex items-center gap-2">
                        <Volume2 className="w-4 h-4 text-green-500" />
                        Audio
                      </div>
                    </SelectItem>
                    <SelectItem value="syphon">
                      <div className="flex items-center gap-2">
                        <Share2 className="w-4 h-4 text-orange-500" />
                        Syphon (macOS)
                      </div>
                    </SelectItem>
                    <SelectItem value="spout">
                      <div className="flex items-center gap-2">
                        <Layers className="w-4 h-4 text-cyan-500" />
                        Spout (Windows)
                      </div>
                    </SelectItem>
                  </SelectContent>
                </Select>
              </div>

              {editingOutput.type === "display" && (
                <>
                  <div className="space-y-1">
                    <Label htmlFor="display-select" className="text-xs">
                      Monitor
                    </Label>
                    <Select
                      value={String(editingOutput.displayIndex ?? 0)}
                      onValueChange={(value) =>
                        setEditingOutput({
                          ...editingOutput,
                          displayIndex: parseInt(value),
                        })
                      }
                    >
                      <SelectTrigger className="h-8">
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
                    <Label htmlFor="fullscreen" className="text-xs">
                      Fullscreen
                    </Label>
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
                <div className="space-y-1">
                  <Label htmlFor="ndi-name" className="text-xs">
                    NDI Name
                  </Label>
                  <Input
                    id="ndi-name"
                    value={editingOutput.ndiName ?? ""}
                    onChange={(e) =>
                      setEditingOutput({
                        ...editingOutput,
                        ndiName: e.target.value,
                      })
                    }
                    className="h-8"
                  />
                </div>
              )}

              {editingOutput.type === "syphon" && (
                <div className="space-y-1">
                  <Label htmlFor="syphon-name" className="text-xs">
                    Syphon Server Name
                  </Label>
                  <Input
                    id="syphon-name"
                    value={editingOutput.syphonName ?? ""}
                    onChange={(e) =>
                      setEditingOutput({
                        ...editingOutput,
                        syphonName: e.target.value,
                      })
                    }
                    className="h-8"
                  />
                  <p className="text-xs text-muted-foreground">
                    Other apps will see this name when receiving Syphon frames
                  </p>
                </div>
              )}

              {editingOutput.type === "spout" && (
                <div className="space-y-1">
                  <Label htmlFor="spout-name" className="text-xs">
                    Spout Sender Name
                  </Label>
                  <Input
                    id="spout-name"
                    value={editingOutput.spoutName ?? ""}
                    onChange={(e) =>
                      setEditingOutput({
                        ...editingOutput,
                        spoutName: e.target.value,
                      })
                    }
                    className="h-8"
                  />
                  <p className="text-xs text-muted-foreground">
                    Other apps will see this name when receiving Spout frames
                  </p>
                </div>
              )}

              {editingOutput.type === "audio" && (
                <div className="space-y-1">
                  <Label htmlFor="audio-driver" className="text-xs">
                    Audio Driver
                  </Label>
                  <Select
                    value={editingOutput.audioDriver ?? "auto"}
                    onValueChange={(value) =>
                      setEditingOutput({
                        ...editingOutput,
                        audioDriver: value as AudioDriver,
                      })
                    }
                  >
                    <SelectTrigger className="h-8">
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
            <Button
              variant="outline"
              size="sm"
              onClick={() => setEditingOutput(null)}
            >
              Cancel
            </Button>
            <Button size="sm" onClick={handleSaveOutput}>
              Save
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </div>
  );
}
