import { createTheme, Shadows } from '@mui/material/styles';
import { red } from '@mui/material/colors';
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
/** MUI's default error.main, for the error-state underline glow */
const errorRed = red[700];
/** Active and hovered nav bar page, darkened like the YellowFruit 2/3 nav bar */
export const yfNavActiveTint = 'rgba(0, 0, 0, 0.1)';
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

/**
 * Materialize text inputs are an underline that gains a 1px glow on focus, instead of MUI's 2px bar.
 * Gold rather than yellow so the focus line meets 3:1 contrast on white.
 */
const underlineOverrides = {
  '&::before': { borderBottomColor: mzInputBorder },
  '&::after': { borderBottomWidth: 1, borderBottomColor: yfGold, boxShadow: `0 1px 0 0 ${yfGold}` },
  '&.Mui-error::after': { borderBottomColor: errorRed, boxShadow: `0 1px 0 0 ${errorRed}` },
};

const yfTheme = createTheme({
  palette: {
    primary: {
      main: yfYellow,
      light: yfYellowLight,
      // MUI uses this as the hover background of contained buttons, so it stays a yellow; text uses yfGold directly
      dark: '#f57f17',
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
          variants: [
            { props: { color: 'standard' }, style: { '&.Mui-selected': { color: yfGold } } },
            { props: { color: 'primary' }, style: { '&.Mui-selected': { color: yfGold } } },
          ],
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
        root: { backgroundColor: '#fafafa' },
      },
    },
    // Materialize fields are an underline with the label above the text: MUI's standard variant
    MuiTextField: { defaultProps: { variant: 'standard' } },
    MuiFormControl: { defaultProps: { variant: 'standard' } },
    MuiSelect: { defaultProps: { variant: 'standard' } },
    MuiPickersTextField: { defaultProps: { variant: 'standard' } },
    MuiInput: { styleOverrides: { underline: underlineOverrides } },
    MuiPickersInput: { styleOverrides: { root: underlineOverrides } },
    MuiFormLabel: {
      styleOverrides: {
        root: {
          '&.Mui-focused:not(.Mui-error)': { color: yfGold },
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
