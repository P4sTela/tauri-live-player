import { useState } from "react";
import {
  Plus,
  Trash2,
  Edit2,
  FileVideo,
  FileAudio,
  GripVertical,
  Monitor,
  Radio,
  Volume2,
  Settings2,
} from "lucide-react";
import { open } from "@tauri-apps/plugin-dialog";
import { useProjectStore } from "../../stores/projectStore";
import { Button } from "../ui/button";
import { Input } from "../ui/input";
import { Label } from "../ui/label";
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
import { Card, CardContent } from "../ui/card";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "../ui/tabs";
import { OutputManager } from "../output/OutputManager";
import { generateId } from "../../lib/utils";
import { cn } from "../../lib/utils";
import type { Cue, MediaItem } from "../../types";

export function EditView() {
  const {
    project,
    addCue,
    updateCue,
    removeCue,
    addItemToCue,
    updateItem,
    removeItem,
  } = useProjectStore();

  const [selectedCueId, setSelectedCueId] = useState<string | null>(null);
  const [editingCue, setEditingCue] = useState<Cue | null>(null);
  const [isAddingItem, setIsAddingItem] = useState(false);

  const selectedCue = project?.cues.find((c) => c.id === selectedCueId);

  // Get outputs for video/audio selection
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
    setSelectedCueId(newCue.id);
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
    if (selectedCueId === id) {
      setSelectedCueId(null);
    }
    removeCue(id);
  };

  const handleAddMediaItem = async (type: "video" | "audio") => {
    if (!selectedCueId) return;

    const extensions =
      type === "video"
        ? ["mp4", "mov", "avi", "mkv", "webm"]
        : ["wav", "mp3", "aac", "flac", "ogg"];

    const file = await open({
      multiple: false,
      filters: [
        {
          name: type === "video" ? "Video" : "Audio",
          extensions,
        },
      ],
    });

    if (file) {
      const fileName =
        file.split("/").pop() || file.split("\\").pop() || "Untitled";

      // Auto-select first available output of matching type
      const availableOutputs = type === "video" ? videoOutputs : audioOutputs;
      const defaultOutputId = availableOutputs[0]?.id ?? "";

      const newItem: MediaItem = {
        id: generateId(),
        type,
        name: fileName,
        path: file,
        outputId: defaultOutputId,
      };
      addItemToCue(selectedCueId, newItem);
    }
    setIsAddingItem(false);
  };

  const handleDeleteItem = (cueId: string, itemId: string) => {
    removeItem(cueId, itemId);
  };

  const handleItemOutputChange = (itemId: string, outputId: string) => {
    if (selectedCueId) {
      updateItem(selectedCueId, itemId, { outputId });
    }
  };

  const getOutputIcon = (outputId: string) => {
    const output = project?.outputs.find((o) => o.id === outputId);
    if (!output) return null;
    switch (output.type) {
      case "display":
        return <Monitor className="w-3 h-3" />;
      case "ndi":
        return <Radio className="w-3 h-3" />;
      case "audio":
        return <Volume2 className="w-3 h-3" />;
    }
  };

  const getOutputName = (outputId: string) => {
    const output = project?.outputs.find((o) => o.id === outputId);
    return output?.name ?? "No output";
  };

  if (!project) {
    return (
      <div className="flex items-center justify-center h-full text-muted-foreground">
        No project loaded
      </div>
    );
  }

  return (
    <div className="flex h-full">
      {/* Left Panel - Cue List & Outputs */}
      <div className="w-1/3 border-r flex flex-col">
        <Tabs defaultValue="cues" className="flex flex-col h-full">
          <div className="p-2 border-b bg-muted/30">
            <TabsList className="w-full">
              <TabsTrigger value="cues" className="flex-1">
                Cues
              </TabsTrigger>
              <TabsTrigger value="outputs" className="flex-1">
                <Settings2 className="w-4 h-4 mr-1" />
                Outputs
              </TabsTrigger>
            </TabsList>
          </div>

          <TabsContent value="cues" className="flex-1 flex flex-col m-0">
            <div className="p-2 border-b flex justify-end">
              <Button size="sm" onClick={handleAddCue}>
                <Plus className="w-4 h-4 mr-1" />
                Add Cue
              </Button>
            </div>

            <div className="flex-1 overflow-auto">
              {project.cues.length === 0 ? (
                <div className="p-4 text-center text-muted-foreground text-sm">
                  No cues yet. Click "Add Cue" to create one.
                </div>
              ) : (
                <div className="p-2 space-y-1">
                  {project.cues.map((cue, index) => (
                    <div
                      key={cue.id}
                      className={cn(
                        "group flex items-center gap-2 p-3 rounded-md cursor-pointer transition-colors",
                        "hover:bg-muted/50",
                        selectedCueId === cue.id
                          ? "bg-primary/10 border border-primary"
                          : "border border-transparent",
                      )}
                      onClick={() => setSelectedCueId(cue.id)}
                    >
                      <GripVertical className="w-4 h-4 text-muted-foreground opacity-0 group-hover:opacity-100 cursor-grab" />
                      <span className="w-6 text-sm text-muted-foreground">
                        {index + 1}
                      </span>
                      <span className="flex-1 truncate">{cue.name}</span>
                      <span className="text-xs text-muted-foreground">
                        {cue.items.length}
                      </span>
                      <Button
                        variant="ghost"
                        size="icon"
                        className="h-7 w-7 opacity-0 group-hover:opacity-100"
                        onClick={(e) => {
                          e.stopPropagation();
                          handleEditCue(cue);
                        }}
                      >
                        <Edit2 className="w-3 h-3" />
                      </Button>
                      <Button
                        variant="ghost"
                        size="icon"
                        className="h-7 w-7 opacity-0 group-hover:opacity-100 text-destructive"
                        onClick={(e) => {
                          e.stopPropagation();
                          handleDeleteCue(cue.id);
                        }}
                      >
                        <Trash2 className="w-3 h-3" />
                      </Button>
                    </div>
                  ))}
                </div>
              )}
            </div>
          </TabsContent>

          <TabsContent value="outputs" className="flex-1 overflow-auto p-4 m-0">
            <OutputManager />
          </TabsContent>
        </Tabs>
      </div>

      {/* Right Panel - Cue Details & Items */}
      <div className="flex-1 flex flex-col">
        {selectedCue ? (
          <>
            {/* Cue Header */}
            <div className="p-3 border-b bg-muted/30">
              <div className="flex items-center justify-between">
                <h2 className="font-medium">{selectedCue.name}</h2>
                <div className="flex items-center gap-2">
                  <Button
                    size="sm"
                    variant="outline"
                    onClick={() => setIsAddingItem(true)}
                  >
                    <Plus className="w-4 h-4 mr-1" />
                    Add Media
                  </Button>
                </div>
              </div>
              <div className="flex gap-4 mt-2 text-sm text-muted-foreground">
                <label className="flex items-center gap-2">
                  <input
                    type="checkbox"
                    checked={selectedCue.loop}
                    onChange={(e) =>
                      updateCue(selectedCue.id, { loop: e.target.checked })
                    }
                    className="rounded"
                  />
                  Loop
                </label>
                <label className="flex items-center gap-2">
                  <input
                    type="checkbox"
                    checked={selectedCue.autoAdvance}
                    onChange={(e) =>
                      updateCue(selectedCue.id, {
                        autoAdvance: e.target.checked,
                      })
                    }
                    className="rounded"
                  />
                  Auto Advance
                </label>
              </div>
            </div>

            {/* Media Items */}
            <div className="flex-1 overflow-auto p-4">
              {selectedCue.items.length === 0 ? (
                <div className="flex flex-col items-center justify-center h-full text-muted-foreground">
                  <p className="mb-4">No media items in this cue.</p>
                  <Button
                    variant="outline"
                    onClick={() => setIsAddingItem(true)}
                  >
                    <Plus className="w-4 h-4 mr-2" />
                    Add Media
                  </Button>
                </div>
              ) : (
                <div className="space-y-2">
                  {selectedCue.items.map((item) => {
                    const availableOutputs =
                      item.type === "video" ? videoOutputs : audioOutputs;

                    return (
                      <Card key={item.id}>
                        <CardContent className="p-3">
                          <div className="flex items-center gap-3">
                            <div
                              className={cn(
                                "w-10 h-10 rounded flex items-center justify-center",
                                item.type === "video"
                                  ? "bg-blue-500/20 text-blue-500"
                                  : "bg-green-500/20 text-green-500",
                              )}
                            >
                              {item.type === "video" ? (
                                <FileVideo className="w-5 h-5" />
                              ) : (
                                <FileAudio className="w-5 h-5" />
                              )}
                            </div>
                            <div className="flex-1 min-w-0">
                              <div className="font-medium truncate">
                                {item.name}
                              </div>
                              <div className="text-xs text-muted-foreground truncate">
                                {item.path}
                              </div>
                            </div>

                            {/* Output Selection */}
                            <Select
                              value={item.outputId || ""}
                              onValueChange={(value) =>
                                handleItemOutputChange(item.id, value)
                              }
                            >
                              <SelectTrigger className="w-40">
                                <SelectValue placeholder="Select output">
                                  {item.outputId ? (
                                    <span className="flex items-center gap-2">
                                      {getOutputIcon(item.outputId)}
                                      {getOutputName(item.outputId)}
                                    </span>
                                  ) : (
                                    "Select output"
                                  )}
                                </SelectValue>
                              </SelectTrigger>
                              <SelectContent>
                                {availableOutputs.length === 0 ? (
                                  <div className="px-2 py-1 text-sm text-muted-foreground">
                                    No{" "}
                                    {item.type === "video" ? "video" : "audio"}{" "}
                                    outputs
                                  </div>
                                ) : (
                                  availableOutputs.map((output) => (
                                    <SelectItem
                                      key={output.id}
                                      value={output.id}
                                    >
                                      <span className="flex items-center gap-2">
                                        {output.type === "display" && (
                                          <Monitor className="w-4 h-4" />
                                        )}
                                        {output.type === "ndi" && (
                                          <Radio className="w-4 h-4" />
                                        )}
                                        {output.type === "audio" && (
                                          <Volume2 className="w-4 h-4" />
                                        )}
                                        {output.name}
                                      </span>
                                    </SelectItem>
                                  ))
                                )}
                              </SelectContent>
                            </Select>

                            <Button
                              variant="ghost"
                              size="icon"
                              className="text-destructive"
                              onClick={() =>
                                handleDeleteItem(selectedCue.id, item.id)
                              }
                            >
                              <Trash2 className="w-4 h-4" />
                            </Button>
                          </div>
                        </CardContent>
                      </Card>
                    );
                  })}
                </div>
              )}
            </div>
          </>
        ) : (
          <div className="flex items-center justify-center h-full text-muted-foreground">
            Select a cue to edit
          </div>
        )}
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
                  <input
                    type="checkbox"
                    checked={editingCue.loop}
                    onChange={(e) =>
                      setEditingCue({ ...editingCue, loop: e.target.checked })
                    }
                    className="rounded"
                  />
                  Loop
                </label>
                <label className="flex items-center gap-2">
                  <input
                    type="checkbox"
                    checked={editingCue.autoAdvance}
                    onChange={(e) =>
                      setEditingCue({
                        ...editingCue,
                        autoAdvance: e.target.checked,
                      })
                    }
                    className="rounded"
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

      {/* Add Media Dialog */}
      <Dialog open={isAddingItem} onOpenChange={setIsAddingItem}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>Add Media</DialogTitle>
          </DialogHeader>
          <div className="grid grid-cols-2 gap-4">
            <Button
              variant="outline"
              className="h-24 flex-col gap-2"
              onClick={() => handleAddMediaItem("video")}
            >
              <FileVideo className="w-8 h-8 text-blue-500" />
              <span>Video</span>
            </Button>
            <Button
              variant="outline"
              className="h-24 flex-col gap-2"
              onClick={() => handleAddMediaItem("audio")}
            >
              <FileAudio className="w-8 h-8 text-green-500" />
              <span>Audio</span>
            </Button>
          </div>
        </DialogContent>
      </Dialog>
    </div>
  );
}
