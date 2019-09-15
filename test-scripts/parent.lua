local root = remodel.readModelFile("test-models/folder-and-value.rbxmx")[1]
assert(root.Name == "Root")

-- TODO: Models are currently parented to a DataModel silently still
-- assert(root.Parent == nil)

local child1 = root:GetChildren()[1]
assert(child1 ~= nil)
assert(child1.Parent.Name == "Root")