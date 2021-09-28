<script>
import { getImageFile } from "../images";

export default {
  name: 'ItemIcon',
  props: {
    item: String,
    count: {
      type: Number,
      default: 0,
    },
    noCount: {
      type: Boolean,
      default: false,
    },
    transparent: {
      type: Boolean,
      default: false,
    },
  },

  setup(props, context) {
    return {
      style: () => {
        const itemFile = getImageFile(props.item);
        return {
          display: `inline-block`,
          position: "relative",
          backgroundImage: `url(${itemFile.url})`,
          backgroundSize: 32 * itemFile.widthFactor + 'px ' + 32 * itemFile.heightFactor + 'px',
          width: `32px`,
          height: `32px`,
        };
      },
    }
  }
}
</script>

<template>
  <div :style="style()" :class="[count === 0 && !noCount || transparent ? 'transparent' : '']">
    <div v-if="!noCount && 0 < count" class="overlay noselect">
    {{ count }}
    </div>
  </div>
</template>
