<script>
import fuelBack from "../../img/fuel-back.png";
import { ref } from "vue";
import rightarrow from "../../img/rightarrow.png";
import { getImageFile } from "../images";
import ItemIcon from "./ItemIcon.vue";

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
      recipeDraw: ref(false),
      tootipZIndex: ref(10000),
      fuelBack,
      getImageFile,
      rightarrow,
      left: ref(0),
      bottom: ref(0),
      title: ref(""),
      text: ref(""),
      recipe: ref({}),
    }
  }
}
</script>

<template>
  <div v-if="visible" ref="tip" class="noselect tooltip"
    :style="{zIndex: tootipZIndex, left: left + 'px', bottom: bottom + 'px'}"
  >
    <div v-if="!recipeDraw" v-html="text" />
    <div v-else>
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
  </div>
</template>
