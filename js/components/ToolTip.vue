<script>
import fuelBack from "../../img/fuel-back.png";
import { ref } from "vue";
import rightarrow from "../../img/rightarrow.png";
import { getImageFile } from "../images";
import ItemIcon from "./ItemIcon.vue";

export const HTMLDraw = 0;
export const RecipeDraw = 1;
export const ResearchDraw = 2;

export default {
  name: 'ToolTip',
  components: {
    ItemIcon,
  },
  props: {
    visible: false,
    items: Array,
    burnerEnergy: Number,
  },

  setup(props, context) {
    return {
      visible: ref(false),
      drawMode: ref(HTMLDraw),
      tootipZIndex: ref(10000),
      owner: ref(""),
      fuelBack,
      getImageFile,
      rightarrow,
      left: ref(0),
      bottom: ref(0),
      title: ref(""),
      text: ref(""),
      recipe: ref({}),
      technology: ref({}),

      HTMLDraw,
      RecipeDraw,
      ResearchDraw,
    }
  }
}
</script>

<template>
  <div v-if="visible" ref="tip" class="noselect tooltip"
    :style="{zIndex: tootipZIndex, left: left + 'px', bottom: bottom + 'px'}"
  >
    <div v-if="drawMode === HTMLDraw" v-html="text" />
    <div v-else-if="drawMode === RecipeDraw">
      <div v-for="k, item in recipe.output" :key="k" style="display: inline-block; width = 10%">
        {{item}}
      </div>
      <div>
        Time: {{recipe.recipe_time * 0.05}}s
      </div>
      <div class="recipe-box" style="width: 200px">
        <span style="display: inline-block; width: 50%">
          <span v-for="count, item in recipe.input" :key="item">
            <item-icon :item="item" :count="count"></item-icon>
          </span>
        </span>
        <img :src="rightarrow" style="width: 20px; height: 32px">
        <span v-for="count, item in recipe.output" :key="item" style="display: inline-block; width = 10%">
          <item-icon :item="item" :count="count"></item-icon>
        </span>
      </div>
    </div>
    <div v-else>
      <div style="display: inline-block; width = 10%">
        {{ technology.tag }}
        {{ technology.unlocked ? "(Unlocked)" : "" }}
      </div>
      <div class="recipe-box" style="width: 200px">
        <span style="display: inline-block; width: 50%">
          <span v-for="count, item in technology.input" :key="item">
            <item-icon :item="item" :count="count * technology.steps"></item-icon>
          </span>
        </span>
      </div>
      <div>
        Unlocks:
      </div>
      <div>
        <span style="display: inline-block; width: 50%">
          <span v-for="item in technology.unlocks" :key="item">
            <item-icon :item="item" :noCount="true"></item-icon>
          </span>
        </span>
      </div>
    </div>
  </div>
</template>
