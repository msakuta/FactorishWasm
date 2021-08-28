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
  },

  setup(props, context) {
    const itemFile = getImageFile(props.item);
    return {
      itemFile,
      count: props.count,

      style: {
        display: `inline-block`,
        position: "relative",
        backgroundImage: `url(${itemFile.url})`,
        backgroundSize: 32 * itemFile.widthFactor + 'px ' + 32 * itemFile.heightFactor + 'px',
        width: `32px`,
        height: `32px`,
      },
    }
  }
}
</script>

<template>
  <div :style="style">
    <div class="overlay noselect">
    {{ count }}
    </div>
  </div>
</template>
