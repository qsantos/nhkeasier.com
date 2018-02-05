from django.contrib import admin

from .models import Story

class StoryAdmin(admin.ModelAdmin):
    list_display = ('title', 'published')
    list_filter = ['published']

admin.site.register(Story, StoryAdmin)
