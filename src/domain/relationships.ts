import type {
  BibleRelationshipSnapshot,
  BibleRelationshipType,
  KnowledgeTargetType,
} from "../api/projectBible";

export interface RelationshipDefinition {
  id: BibleRelationshipType;
  labelKey: string;
  reverseKey: string;
}

const relationshipIds: BibleRelationshipType[] = ["related", "knows", "family", "allied", "enemy", "member_of", "leads", "owns", "located_in", "created_by", "participates_in", "caused", "follows", "uses_system", "depends_on", "affects", "created_by_system"];
export const relationshipDefinitions: RelationshipDefinition[] = relationshipIds.map((id) => ({ id, labelKey: `relationships.${id}.forward`, reverseKey: `relationships.${id}.reverse` }));

export function isRelationshipTypeAllowed(
  sourceType: KnowledgeTargetType,
  targetType: KnowledgeTargetType,
  relationshipType: BibleRelationshipType,
) {
  const characterOrFaction = (type: KnowledgeTargetType) =>
    type === "character" || type === "faction";

  switch (relationshipType) {
    case "related": return true;
    case "knows":
    case "allied":
    case "enemy": return characterOrFaction(sourceType) && characterOrFaction(targetType);
    case "family": return sourceType === "character" && targetType === "character";
    case "member_of":
    case "leads": return characterOrFaction(sourceType) && targetType === "faction";
    case "owns": return targetType === "item";
    case "located_in": return targetType === "location";
    case "created_by": return sourceType === "item" && characterOrFaction(targetType);
    case "participates_in":
    case "caused": return targetType === "event";
    case "follows": return sourceType === "event" && targetType === "event";
    case "uses_system": return targetType === "world_system";
    case "depends_on":
    case "affects":
    case "created_by_system": return sourceType === "world_system" || targetType === "world_system";
  }
}

export function relationshipDisplayLabel(
  relationship: Pick<BibleRelationshipSnapshot, "relationshipType" | "sourceType" | "sourceId">,
  viewedType: KnowledgeTargetType,
  viewedId: string,
  translate: (key: string) => string,
) {
  const definition = relationshipDefinitions.find(
    (item) => item.id === relationship.relationshipType,
  );
  const outgoing = relationship.sourceType === viewedType && relationship.sourceId === viewedId;
  const key = outgoing ? definition?.labelKey : definition?.reverseKey;
  return key ? translate(key) : relationship.relationshipType;
}
