from django import forms


class ContactForm(forms.Form):
    from_email = forms.EmailField(label='Your email')
    subject = forms.CharField(required=False)
    message = forms.CharField(widget=forms.Textarea)

    def __init__(self, *args, **kwargs):
        super().__init__(*args, **kwargs)
        self.fields['from_email'].widget.attrs['placeholder'] = 'john.doe@example.com'
