from django import forms

class ContactForm(forms.Form):
    from_email = forms.EmailField(label='Your email', widget=forms.TextInput(attrs={'placeholder': 'john.doe@example.com'}))
    subject = forms.CharField(required=False)
    message = forms.CharField(widget=forms.Textarea)
