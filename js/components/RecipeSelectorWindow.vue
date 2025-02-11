<script>
import CloseButton from "./CloseButton.vue";
import ItemIcon from './ItemIcon.vue';
import itemBack from "../../img/item-back.png";
import { nextTick, ref } from "vue";

export default {
  components: {
    CloseButton,
    ItemIcon,
  },

  props: {
    dragWindowMouseDown: Function,
    showRecipeSelect: Function,
    recipeClickHandler: Function,
    recipeMouseEnterHandler: Function,
    recipeMouseLeaveHandler: Function,
    bringToTop: Function,
  },

  setup(props, context) {
    const {
      dragWindowMouseDown,
      recipeClickHandler,
      recipeMouseEnterHandler,
      recipeMouseLeaveHandler,
    } = props;

    const visible = ref(false);
    const recipes = ref([]);

    const firstOutput = (recipe) => {
      let result = "";
      if (!recipe || !recipe.output) {
        return "";
      }
      recipe.output.forEach((v, k) => {
          console.log(`  ${k}: ${v}`);
          result = k;
      });
      return result;
    }

    return {
      visible,
      left: ref(0),
      top: ref(0),
      recipes,
      itemBack,

      firstOutput,

      zIndex: ref(0),
      dragWindowMouseDown,

      inventoryDragStart: null,
      close(){
        console.log("Handling click: " + visible.value);
        visible.value = !visible.value;
      },

      onClickRecipe(i, evt){ recipeClickHandler(recipes.value, i, evt); },
      onMouseEnterRecipe(i, evt){ recipeMouseEnterHandler(recipes.value, i, evt); },
      onMouseLeaveRecipe(i, evt){ recipeMouseLeaveHandler(recipes.value, i, evt); },
    };
  },

  methods: {
    dragWindow(evt){
      this.dragWindowMouseDown(
        evt,
        this.$refs.root,
        this,
        this.inventoryDragStart,
        (x, y) => {
          this.left = x;
          this.top = y;
        }
      )
    },

    // Place a window element at the center, for Vue component.
    placeCenter() {
      // Defer one tick to allow DOM element to be created
      nextTick(() => {
        if (!this.$refs.root) return;
        var elemRect = this.$refs.root.getBoundingClientRect();
        var bodyRect = document.body.getBoundingClientRect();
        this.left = (bodyRect.width - elemRect.width) / 2;
        this.top = (bodyRect.height - elemRect.height) / 2;
      });
    },
  },
};
</script>

<template>
  <div v-if="visible" ref="root"
    :class="['noselect', 'recipeSelector']"
    :style="{left: `${left}px`, top: `${top}px`, zIndex}"
    @click="bringToTop"
  >
    <div ref="recipeTitle" class="inventoryTitle" @mousedown="dragWindow">Select a recipe</div>
    <close-button @click="close"></close-button>
    <div ref="recipeClient">
      <div style="vertical-align: middle;">
        <div v-for="i in Math.ceil((1 + recipes.length) / 10) * 10"
          :key="i"
          class="itemBack"
          @click="evt => onClickRecipe(i-1, evt, false)"
          @contextmenu="evt => onClickRecipe(i-1, evt, true)"
          @mouseenter="evt => onMouseEnterRecipe(i-1, evt)"
          @mouseleave="evt => onMouseLeaveRecipe(i-1, evt)"
          :style="{backgroundColor: `#ffffff`, backgroundImage: `url(${itemBack})`}"
        >
          <template v-if="firstOutput(recipes[i-1])">
            <item-icon
              :item="firstOutput(recipes[i-1])"
              :noCount="true"
            />
          </template>
        </div>
      </div>
    </div>
  </div>
</template>

<style>

</style>