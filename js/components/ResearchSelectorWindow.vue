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
    researchClickHandler: Function,
    researchMouseEnterHandler: Function,
    researchMouseLeaveHandler: Function,
    bringToTop: Function,
  },

  setup(props, context) {
    const {
      dragWindowMouseDown,
      researchClickHandler,
      researchMouseEnterHandler,
      researchMouseLeaveHandler,
    } = props;

    const visible = ref(false);
    const technologies = ref([]);

    return {
      visible,
      left: ref(0),
      top: ref(0),
      research: ref({}),
      technologies,
      itemBack,

      zIndex: ref(0),
      dragWindowMouseDown,

      inventoryDragStart: null,
      close(){
        console.log("Handling click: " + visible.value);
        visible.value = !visible.value;
      },

      onClickResearch(i, evt){ researchClickHandler(technologies.value, i, evt); },
      onMouseEnterResearch(i, evt){ researchMouseEnterHandler(technologies.value, i, evt); },
      onMouseLeaveResearch(i, evt){ researchMouseLeaveHandler(technologies.value, i, evt); },
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
    <div ref="recipeTitle" class="inventoryTitle" @mousedown="dragWindow">Research</div>
    <close-button @click="close"></close-button>
    <div ref="recipeClient">
      <div style="vertical-align: middle;">
        <div>
          <h4>
            Current research:
          </h4>
          <div v-if="research">
            {{ research.technology_name }}
            ({{ research.progress * 100 }}%)
            <div class="progressBarBack">
              <div class="progressBar" :style="{width: `${research.progress * 100}%`}"></div>
            </div>
          </div>
          <div v-else>
            None
          </div>
        </div>
        <div>
          <h4>
            Select a research:
          </h4>
          <div v-for="i in Math.ceil((1 + technologies.length) / 10) * 10"
            :key="i"
            class="itemBack"
            @click="evt => onClickResearch(i-1, evt, false)"
            @contextmenu="evt => onClickResearch(i-1, evt, true)"
            @mouseenter="evt => onMouseEnterResearch(i-1, evt)"
            @mouseleave="evt => onMouseLeaveResearch(i-1, evt)"
            :style="{backgroundColor: `#ffffff`, backgroundImage: `url(${itemBack})`}"
          >
            <template v-if="i-1 < technologies.length">
              <item-icon
                :item="technologies[i-1].image"
                :noCount="true"
              />
            </template>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>

<style>

</style>