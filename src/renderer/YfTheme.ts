import { createTheme, Shadows } from '@mui/material/styles';
import type {} from '@mui/x-date-pickers/themeAugmentation';

// Styled after Materialize 1.0, which YellowFruit 2.x/3.x was built on. Values come from its Sass sources.

// The yellow from YellowFruit 2.x/3.x, whose nav bar was Materialize "yellow darken-3"
const yfYellow = '#f9a825';
const yfYellowLight = '#fbc02d';
/** Yellow text is unreadable on white, so text-style uses of the primary color get this darker gold (5:1 on white) */
const yfGold = '#946400';

/** Materialize's grey for input underlines and placeholder text */
const mzInputBorder = '#9e9e9e';
/** Materialize's background for toasts and tooltips */
const mzDarkSurface = '#323232';
// Materialize's .z-depth-* shadows
const mzDepth1 =
  '0 2px 2px 0 rgba(0,0,0,0.14), 0 3px 1px -2px rgba(0,0,0,0.12), 0 1px 5px 0 rgba(0,0,0,0.2)';
const mzDepth1Half =
  '0 3px 3px 0 rgba(0,0,0,0.14), 0 1px 7px 0 rgba(0,0,0,0.12), 0 3px 1px -1px rgba(0,0,0,0.2)';
const mzDepth2 = '0 4px 5px 0 rgba(0,0,0,0.14), 0 1px 10px 0 rgba(0,0,0,0.12), 0 2px 4px -1px rgba(0,0,0,0.3)';
const mzDepth3 = '0 8px 17px 2px rgba(0,0,0,0.14), 0 3px 14px 2px rgba(0,0,0,0.12), 0 5px 5px -3px rgba(0,0,0,0.2)';
const mzDepth4 = '0 16px 24px 2px rgba(0,0,0,0.14), 0 6px 30px 5px rgba(0,0,0,0.12), 0 8px 10px -7px rgba(0,0,0,0.2)';
const mzDepth5 = '0 24px 38px 3px rgba(0,0,0,0.14), 0 9px 46px 8px rgba(0,0,0,0.12), 0 11px 15px -7px rgba(0,0,0,0.2)';

/** MUI's 25 elevation levels, mapped onto Materialize's six depths */
function mzShadows(): Shadows {
  return Array.from({ length: 25 }, (_, elevation) => {
    if (elevation === 0) return 'none';
    if (elevation === 1) return mzDepth1;
    if (elevation === 2) return mzDepth1Half;
    if (elevation <= 4) return mzDepth2;
    if (elevation <= 8) return mzDepth3;
    if (elevation <= 16) return mzDepth4;
    return mzDepth5;
  }) as Shadows;
}

/** Materialize text inputs have only a bottom border, which gains a 1px glow on focus */
const underlineInputOverrides = {
  root: {
    borderRadius: 0,
    '& .MuiOutlinedInput-notchedOutline, & .MuiPickersOutlinedInput-notchedOutline': {
      borderWidth: '0 0 1px 0',
      borderColor: mzInputBorder,
      borderRadius: 0,
      transition: 'box-shadow .3s, border-color .3s',
    },
    '&:hover:not(.Mui-disabled):not(.Mui-error) .MuiOutlinedInput-notchedOutline, &:hover:not(.Mui-disabled):not(.Mui-error) .MuiPickersOutlinedInput-notchedOutline':
      {
        borderColor: 'rgba(0, 0, 0, 0.87)',
      },
    '&.Mui-focused .MuiOutlinedInput-notchedOutline, &.Mui-focused .MuiPickersOutlinedInput-notchedOutline': {
      borderWidth: '0 0 1px 0',
      borderColor: yfYellow,
      boxShadow: `0 1px 0 0 ${yfYellow}`,
    },
    '&.Mui-error .MuiOutlinedInput-notchedOutline, &.Mui-error .MuiPickersOutlinedInput-notchedOutline': {
      boxShadow: 'none',
    },
    '&.Mui-error.Mui-focused .MuiOutlinedInput-notchedOutline, &.Mui-error.Mui-focused .MuiPickersOutlinedInput-notchedOutline':
      {
        boxShadow: '0 1px 0 0 currentColor',
      },
    '&.Mui-disabled .MuiOutlinedInput-notchedOutline, &.Mui-disabled .MuiPickersOutlinedInput-notchedOutline': {
      borderStyle: 'dotted',
    },
  },
};

const yfTheme = createTheme({
  palette: {
    primary: {
      main: yfYellow,
      light: yfYellowLight,
      dark: yfGold,
      contrastText: 'rgba(0, 0, 0, 0.87)',
    },
  },
  shape: { borderRadius: 2 },
  shadows: mzShadows(),
  components: {
    MuiAppBar: {
      styleOverrides: {
        // white on yellow, like the old nav bar
        colorPrimary: { color: '#ffffff', boxShadow: mzDepth1 },
      },
    },
    MuiButton: {
      styleOverrides: {
        root: {
          letterSpacing: '.5px',
          variants: [
            {
              props: { variant: 'text', color: 'primary' },
              style: { color: yfGold },
            },
            {
              props: { variant: 'outlined', color: 'primary' },
              style: { color: yfGold, borderColor: yfYellow },
            },
            {
              props: { variant: 'contained' },
              style: {
                boxShadow: mzDepth1,
                '&:hover': { boxShadow: mzDepth1Half },
                '&:active': { boxShadow: mzDepth1Half },
              },
            },
          ],
        },
      },
    },
    MuiToggleButton: {
      styleOverrides: {
        root: {
          letterSpacing: '.5px',
          '&.Mui-selected': { color: yfGold },
        },
      },
    },
    MuiCard: {
      styleOverrides: {
        root: {
          transition: 'box-shadow .25s',
          variants: [{ props: { variant: 'outlined' }, style: { borderColor: '#ddd' } }],
        },
      },
    },
    MuiDialogActions: {
      styleOverrides: {
        // Materialize's modal footer
        root: { backgroundColor: '#fafafa', padding: '4px 6px' },
      },
    },
    MuiOutlinedInput: { styleOverrides: underlineInputOverrides },
    MuiPickersOutlinedInput: { styleOverrides: underlineInputOverrides },
    MuiInput: {
      styleOverrides: {
        // standard-variant fields: Materialize keeps a 1px line plus a glow on focus instead of MUI's 2px bar
        underline: {
          '&::before': { borderBottomColor: mzInputBorder },
          '&::after': { borderBottomWidth: 1, boxShadow: `0 1px 0 0 ${yfYellow}` },
          '&.Mui-error::after': { boxShadow: '0 1px 0 0 currentColor' },
        },
      },
    },
    MuiFormLabel: {
      styleOverrides: {
        root: {
          '&.Mui-focused:not(.Mui-error)': { color: yfGold },
        },
      },
    },
    // Outlined labels float on the box's top border. With that border gone they would sit on top of whatever is
    // above the field, so in labelled fields the floated label moves into the field's own top padding and the text
    // shifts down to make room. Heights don't change, so labelled and unlabelled fields in a row still line up.
    MuiFormControl: {
      styleOverrides: {
        root: {
          '&:has(> .MuiInputLabel-outlined)': {
            '& .MuiOutlinedInput-input:not(.MuiAutocomplete-input), & .MuiPickersSectionList-root': {
              paddingTop: 22,
              paddingBottom: 11,
            },
            '& .MuiAutocomplete-inputRoot': { paddingTop: 14.5, paddingBottom: 3.5 },
          },
          '&:has(> .MuiInputLabel-outlined.MuiInputLabel-sizeSmall)': {
            '& .MuiOutlinedInput-input:not(.MuiAutocomplete-input), & .MuiPickersSectionList-root': {
              paddingTop: 14,
              paddingBottom: 3,
            },
            '& .MuiAutocomplete-inputRoot': { paddingTop: 11, paddingBottom: 1 },
          },
        },
      },
    },
    MuiInputLabel: {
      styleOverrides: {
        root: {
          '&.MuiInputLabel-outlined.MuiInputLabel-shrink': { transform: 'translate(14px, -1px) scale(0.75)' },
          '&.MuiInputLabel-outlined.MuiInputLabel-shrink.MuiInputLabel-sizeSmall': {
            transform: 'translate(14px, -4px) scale(0.75)',
          },
        },
      },
    },
    MuiChip: {
      styleOverrides: {
        root: {
          variants: [{ props: { variant: 'filled', color: 'default' }, style: { backgroundColor: '#e4e4e4' } }],
        },
      },
    },
    MuiTooltip: {
      styleOverrides: {
        tooltip: {
          backgroundColor: mzDarkSurface,
          padding: '8px 10px',
          fontSize: '0.8125rem',
          lineHeight: 1.2,
        },
        arrow: { color: mzDarkSurface },
      },
    },
    MuiAlert: {
      styleOverrides: {
        filled: { boxShadow: mzDepth1, fontSize: '1rem' },
      },
    },
    MuiTab: {
      styleOverrides: {
        root: {
          '&.Mui-selected': { color: yfGold },
        },
      },
    },
    MuiLink: {
      styleOverrides: {
        root: { color: yfGold },
      },
    },
  },
});

export default yfTheme;
