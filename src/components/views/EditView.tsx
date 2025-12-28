import { useState } from "react";
import {
  Plus,
  Trash2,
  Edit2,
  FileVideo,
  FileAudio,
  Monitor,
  Radio,
  Volume2,
  Settings2,
  X,
  GripVertical,
} from "lucide-react";
import { open } from "@tauri-apps/plugin-dialog";
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
import { Button } from "../ui/button";
import { Input } from "../ui/input";
import { Label } from "../ui/label";
import { Checkbox } from "../ui/checkbox";
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
  DialogFooter,
} from "../ui/dialog";

import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from "../ui/table";
import { OutputManager } from "../output/OutputManager";
import { generateId } from "../../lib/utils";
import { cn } from "../../lib/utils";
import type { Cue, MediaItem, OutputTarget } from "../../types";

// Sortable table row component
interface SortableCueRowProps {
  cue: Cue;
  isSelected: boolean;
  onSelect: (checked: boolean) => void;
  onEdit: () => void;
  onDelete: () => void;
  videoOutputs: OutputTarget[];
  audioOutputs: OutputTarget[];
  onCellClick: (
    cue: Cue,
    output: OutputTarget,
    type: "video" | "audio",
  ) => void;
  onCellClear: (cue: Cue, outputId: string) => void;
  getItemForOutput: (cue: Cue, outputId: string) => MediaItem | undefined;
}

function SortableCueRow({
  cue,
  isSelected,
  onSelect,
  onEdit,
  onDelete,
  videoOutputs,
  audioOutputs,
  onCellClick,
  onCellClear,
  getItemForOutput,
}: SortableCueRowProps) {
  const {
    attributes,
    listeners,
    setNodeRef,
    transform,
    transition,
    isDragging,
  } = useSortable({ id: cue.id });

  const style = {
    transform: CSS.Transform.toString(transform),
    transition,
  };

  return (
    <TableRow
      ref={setNodeRef}
      style={style}
      data-state={isSelected ? "selected" : undefined}
      className={cn(isDragging && "opacity-50 bg-muted")}
    >
      {/* Drag handle */}
      <TableCell className="w-8">
        <div
          {...attributes}
          {...listeners}
          className="cursor-grab active:cursor-grabbing p-1"
        >
          <GripVertical className="w-4 h-4 text-muted-foreground" />
        </div>
      </TableCell>

      {/* Checkbox */}
      <TableCell className="w-8">
        <Checkbox checked={isSelected} onCheckedChange={onSelect} />
      </TableCell>

      {/* Cue name */}
      <TableCell className="font-medium">
        <div
          className="flex items-center gap-2 cursor-pointer hover:text-primary"
          onClick={onEdit}
        >
          <span>{cue.name}</span>
          {cue.loop && (
            <span className="text-xs px-1.5 py-0.5 rounded bg-muted text-muted-foreground">
              Loop
            </span>
          )}
          {cue.autoAdvance && (
            <span className="text-xs px-1.5 py-0.5 rounded bg-muted text-muted-foreground">
              Auto
            </span>
          )}
        </div>
      </TableCell>

      {/* Video output cells */}
      {videoOutputs.map((output) => {
        const item = getItemForOutput(cue, output.id);
        return (
          <TableCell
            key={output.id}
            className={cn(
              "cursor-pointer transition-colors",
              item ? "hover:bg-blue-500/10" : "hover:bg-muted/50",
            )}
            onClick={() => onCellClick(cue, output, "video")}
          >
            {item ? (
              <div className="flex items-center gap-2 group/cell">
                <div className="flex items-center gap-2 px-2 py-1 rounded bg-blue-500/10 text-blue-600 dark:text-blue-400">
                  <FileVideo className="w-4 h-4 shrink-0" />
                  <span className="text-sm truncate max-w-[120px]">
                    {item.name}
                  </span>
                </div>
                <Button
                  variant="ghost"
                  size="icon"
                  className="h-6 w-6 opacity-0 group-hover/cell:opacity-100 shrink-0"
                  onClick={(e) => {
                    e.stopPropagation();
                    onCellClear(cue, output.id);
                  }}
                >
                  <X className="w-3 h-3" />
                </Button>
              </div>
            ) : (
              <span className="text-muted-foreground/40">—</span>
            )}
          </TableCell>
        );
      })}

      {/* Audio output cells */}
      {audioOutputs.map((output) => {
        const item = getItemForOutput(cue, output.id);
        return (
          <TableCell
            key={output.id}
            className={cn(
              "cursor-pointer transition-colors",
              item ? "hover:bg-green-500/10" : "hover:bg-muted/50",
            )}
            onClick={() => onCellClick(cue, output, "audio")}
          >
            {item ? (
              <div className="flex items-center gap-2 group/cell">
                <div className="flex items-center gap-2 px-2 py-1 rounded bg-green-500/10 text-green-600 dark:text-green-400">
                  <FileAudio className="w-4 h-4 shrink-0" />
                  <span className="text-sm truncate max-w-[120px]">
                    {item.name}
                  </span>
                </div>
                <Button
                  variant="ghost"
                  size="icon"
                  className="h-6 w-6 opacity-0 group-hover/cell:opacity-100 shrink-0"
                  onClick={(e) => {
                    e.stopPropagation();
                    onCellClear(cue, output.id);
                  }}
                >
                  <X className="w-3 h-3" />
                </Button>
              </div>
            ) : (
              <span className="text-muted-foreground/40">—</span>
            )}
          </TableCell>
        );
      })}

      {/* Actions */}
      <TableCell className="text-right">
        <div className="flex items-center justify-end gap-1">
          <Button
            variant="ghost"
            size="icon"
            className="h-8 w-8"
            onClick={onEdit}
          >
            <Edit2 className="w-4 h-4" />
          </Button>
          <Button
            variant="ghost"
            size="icon"
            className="h-8 w-8 text-destructive hover:text-destructive"
            onClick={onDelete}
          >
            <Trash2 className="w-4 h-4" />
          </Button>
        </div>
      </TableCell>
    </TableRow>
  );
}

export function EditView() {
  const {
    project,
    addCue,
    updateCue,
    removeCue,
    reorderCues,
    addItemToCue,
    updateItem,
    removeItem,
  } = useProjectStore();

  const [editingCue, setEditingCue] = useState<Cue | null>(null);
  const [selectedCues, setSelectedCues] = useState<Set<string>>(new Set());

  const sensors = useSensors(
    useSensor(PointerSensor),
    useSensor(KeyboardSensor, {
      coordinateGetter: sortableKeyboardCoordinates,
    }),
  );

  // 映像出力と音声出力を分離
  const videoOutputs = project?.outputs.filter((o) => o.type !== "audio") ?? [];
  const audioOutputs = project?.outputs.filter((o) => o.type === "audio") ?? [];

  const handleAddCue = () => {
    const newCue: Cue = {
      id: generateId(),
      name: `Cue ${(project?.cues.length || 0) + 1}`,
      items: [],
      duration: 0,
      loop: false,
      autoAdvance: false,
    };
    addCue(newCue);
  };

  const handleEditCue = (cue: Cue) => {
    setEditingCue({ ...cue });
  };

  const handleSaveCue = () => {
    if (editingCue) {
      updateCue(editingCue.id, editingCue);
      setEditingCue(null);
    }
  };

  const handleDeleteCue = (id: string) => {
    removeCue(id);
    setSelectedCues((prev) => {
      const next = new Set(prev);
      next.delete(id);
      return next;
    });
  };

  const handleSelectCue = (cueId: string, checked: boolean) => {
    setSelectedCues((prev) => {
      const next = new Set(prev);
      if (checked) {
        next.add(cueId);
      } else {
        next.delete(cueId);
      }
      return next;
    });
  };

  const handleSelectAll = (checked: boolean) => {
    if (checked && project) {
      setSelectedCues(new Set(project.cues.map((c) => c.id)));
    } else {
      setSelectedCues(new Set());
    }
  };

  const handleDragEnd = (event: DragEndEvent) => {
    const { active, over } = event;
    if (!over || active.id === over.id || !project) return;

    const oldIndex = project.cues.findIndex((c) => c.id === active.id);
    const newIndex = project.cues.findIndex((c) => c.id === over.id);

    if (oldIndex !== -1 && newIndex !== -1) {
      reorderCues(oldIndex, newIndex);
    }
  };

  // セルクリックでファイル選択
  const handleCellClick = async (
    cue: Cue,
    output: OutputTarget,
    type: "video" | "audio",
  ) => {
    const existingItem = cue.items.find((item) => item.outputId === output.id);

    const extensions =
      type === "video"
        ? ["mp4", "mov", "avi", "mkv", "webm"]
        : ["wav", "mp3", "aac", "flac", "ogg"];

    const file = await open({
      multiple: false,
      filters: [{ name: type === "video" ? "Video" : "Audio", extensions }],
    });

    if (file) {
      const fileName =
        file.split("/").pop() || file.split("\\").pop() || "Untitled";

      if (existingItem) {
        updateItem(cue.id, existingItem.id, { path: file, name: fileName });
      } else {
        const newItem: MediaItem = {
          id: generateId(),
          type,
          name: fileName,
          path: file,
          outputId: output.id,
        };
        addItemToCue(cue.id, newItem);
      }
    }
  };

  // セルからアイテムを削除
  const handleCellClear = (cue: Cue, outputId: string) => {
    const item = cue.items.find((i) => i.outputId === outputId);
    if (item) {
      removeItem(cue.id, item.id);
    }
  };

  // 特定のCue・出力に対応するMediaItemを取得
  const getItemForOutput = (
    cue: Cue,
    outputId: string,
  ): MediaItem | undefined => {
    return cue.items.find((item) => item.outputId === outputId);
  };

  const getOutputIcon = (type: string) => {
    switch (type) {
      case "display":
        return <Monitor className="w-4 h-4" />;
      case "ndi":
        return <Radio className="w-4 h-4" />;
      case "audio":
        return <Volume2 className="w-4 h-4" />;
      default:
        return null;
    }
  };

  if (!project) {
    return (
      <div className="flex items-center justify-center h-full text-muted-foreground">
        No project loaded
      </div>
    );
  }

  const hasOutputs = videoOutputs.length > 0 || audioOutputs.length > 0;
  const allSelected =
    project.cues.length > 0 && selectedCues.size === project.cues.length;
  const someSelected = selectedCues.size > 0 && !allSelected;

  return (
    <div className="flex h-full">
      {/* Main Content - Spreadsheet Table */}
      <div className="flex-1 flex flex-col overflow-hidden">
        {!hasOutputs ? (
          <div className="flex-1 flex items-center justify-center text-muted-foreground">
            <div className="text-center">
              <p className="mb-4">No outputs configured.</p>
              <p className="text-sm">Add outputs in the Outputs panel first.</p>
            </div>
          </div>
        ) : (
          <>
            <div className="flex-1 overflow-auto">
              <DndContext
                sensors={sensors}
                collisionDetection={closestCenter}
                onDragEnd={handleDragEnd}
              >
                <Table>
                  <TableHeader>
                    <TableRow className="hover:bg-transparent border-b">
                      {/* Drag handle column */}
                      <TableHead className="w-8"></TableHead>

                      {/* Checkbox column */}
                      <TableHead className="w-8">
                        <Checkbox
                          checked={
                            allSelected ||
                            (someSelected ? "indeterminate" : false)
                          }
                          onCheckedChange={handleSelectAll}
                        />
                      </TableHead>

                      {/* Cue name column */}
                      <TableHead className="min-w-[200px]">Cue</TableHead>

                      {/* Video output columns */}
                      {videoOutputs.map((output) => (
                        <TableHead key={output.id} className="min-w-[180px]">
                          <div className="flex items-center gap-2">
                            {getOutputIcon(output.type)}
                            <span>{output.name}</span>
                          </div>
                        </TableHead>
                      ))}

                      {/* Audio output columns */}
                      {audioOutputs.map((output) => (
                        <TableHead key={output.id} className="min-w-[180px]">
                          <div className="flex items-center gap-2">
                            {getOutputIcon(output.type)}
                            <span>{output.name}</span>
                          </div>
                        </TableHead>
                      ))}

                      {/* Actions column */}
                      <TableHead className="w-20 text-right">
                        <Button
                          size="sm"
                          variant="outline"
                          onClick={handleAddCue}
                        >
                          <Plus className="w-4 h-4 mr-1" />
                          Add Cue
                        </Button>
                      </TableHead>
                    </TableRow>
                  </TableHeader>

                  <TableBody>
                    {project.cues.length === 0 ? (
                      <TableRow>
                        <TableCell
                          colSpan={
                            videoOutputs.length + audioOutputs.length + 4
                          }
                          className="h-32 text-center text-muted-foreground"
                        >
                          No cues yet. Click "Add Cue" to create one.
                        </TableCell>
                      </TableRow>
                    ) : (
                      <SortableContext
                        items={project.cues.map((c) => c.id)}
                        strategy={verticalListSortingStrategy}
                      >
                        {project.cues.map((cue) => (
                          <SortableCueRow
                            key={cue.id}
                            cue={cue}
                            isSelected={selectedCues.has(cue.id)}
                            onSelect={(checked) =>
                              handleSelectCue(cue.id, checked)
                            }
                            onEdit={() => handleEditCue(cue)}
                            onDelete={() => handleDeleteCue(cue.id)}
                            videoOutputs={videoOutputs}
                            audioOutputs={audioOutputs}
                            onCellClick={handleCellClick}
                            onCellClear={handleCellClear}
                            getItemForOutput={getItemForOutput}
                          />
                        ))}
                      </SortableContext>
                    )}
                  </TableBody>
                </Table>
              </DndContext>
            </div>

            {/* Footer */}
            <div className="border-t px-4 py-2 text-sm text-muted-foreground">
              {selectedCues.size} of {project.cues.length} cue(s) selected.
            </div>
          </>
        )}
      </div>

      {/* Right Sidebar - Outputs */}
      <div className="w-64 border-l flex flex-col">
        <div className="px-3 py-2 border-b bg-muted/30 flex items-center gap-1.5">
          <Settings2 className="w-3.5 h-3.5 text-muted-foreground" />
          <span className="text-sm font-medium">Outputs</span>
        </div>
        <div className="flex-1 overflow-auto p-2">
          <OutputManager />
        </div>
      </div>

      {/* Edit Cue Dialog */}
      <Dialog open={!!editingCue} onOpenChange={() => setEditingCue(null)}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>Edit Cue</DialogTitle>
          </DialogHeader>
          {editingCue && (
            <div className="space-y-4">
              <div>
                <Label htmlFor="cue-name">Name</Label>
                <Input
                  id="cue-name"
                  value={editingCue.name}
                  onChange={(e) =>
                    setEditingCue({ ...editingCue, name: e.target.value })
                  }
                />
              </div>
              <div className="flex gap-4">
                <label className="flex items-center gap-2">
                  <Checkbox
                    checked={editingCue.loop}
                    onCheckedChange={(checked) =>
                      setEditingCue({ ...editingCue, loop: checked as boolean })
                    }
                  />
                  Loop
                </label>
                <label className="flex items-center gap-2">
                  <Checkbox
                    checked={editingCue.autoAdvance}
                    onCheckedChange={(checked) =>
                      setEditingCue({
                        ...editingCue,
                        autoAdvance: checked as boolean,
                      })
                    }
                  />
                  Auto Advance
                </label>
              </div>
            </div>
          )}
          <DialogFooter>
            <Button variant="outline" onClick={() => setEditingCue(null)}>
              Cancel
            </Button>
            <Button onClick={handleSaveCue}>Save</Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </div>
  );
}
