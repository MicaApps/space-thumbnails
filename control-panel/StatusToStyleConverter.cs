using Microsoft.UI.Xaml;
using Microsoft.UI.Xaml.Data;
using System;

namespace SpaceThumbnails.ControlPanel
{
    public class StatusToStyleConverter : IValueConverter
    {
        public Style HighlightStyle { get; set; }
        public Style NormalStyle { get; set; }

        public object Convert(object value, Type targetType, object parameter, string language)
        {
            if (value is bool isEnabled && parameter is string mode)
            {
                bool highlight = false;
                if (mode == "Enable") highlight = isEnabled;
                else if (mode == "Restore") highlight = !isEnabled;

                if (highlight) return HighlightStyle;
            }
            return NormalStyle;
        }

        public object ConvertBack(object value, Type targetType, object parameter, string language)
        {
            throw new NotImplementedException();
        }
    }
}
