import { useEffect, useRef } from 'react';
import { ArrowDown, ArrowUpRight, Check, CheckCheck, CornerDownLeft, ListTodo, Plus, Trash2 } from 'lucide-react';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { Badge } from '@/components/ui/badge';
import { Separator } from '@/components/ui/separator';
import { Progress } from '@/components/ui/progress';
import { Alert, AlertDescription } from '@/components/ui/alert';
import { AlertDialog, AlertDialogCancel, AlertDialogContent, AlertDialogDescription, AlertDialogFooter, AlertDialogHeader, AlertDialogTitle, AlertDialogTrigger } from '@/components/ui/alert-dialog';

type Filter = 'all' | 'active' | 'completed';
type Todo = { id: string; title: string; completed: boolean };
export type InitialState = { todos: Todo[]; filter: Filter; error: string | null };

function HiddenFilter({ filter }: { filter: Filter }) {
  return <input type="hidden" name="filter" value={filter} />;
}

function AddTodo({ filter }: { filter: Filter }) {
  const input = useRef<HTMLInputElement>(null);
  useEffect(() => {
    function focusInput(event: KeyboardEvent) {
      const target = event.target;
      if (event.key !== '/' || event.metaKey || event.ctrlKey || event.altKey || event.isComposing) return;
      if (target instanceof HTMLElement && (target.isContentEditable || target.closest('input, textarea, select, [role="alertdialog"]'))) return;
      event.preventDefault();
      input.current?.focus();
    }
    window.addEventListener('keydown', focusInput);
    return () => window.removeEventListener('keydown', focusInput);
  }, []);
  return <form className="add-form" action="/todos/add" method="post">
    <HiddenFilter filter={filter} />
    <label className="sr-only" htmlFor="todo-title">新しいTodo</label>
    <div className="input-wrap">
      <Plus className="input-icon" aria-hidden="true" />
      <Input ref={input} id="todo-title" name="title" required maxLength={200} autoComplete="off" placeholder="次にやることを入力…" aria-describedby="input-hint" />
      <kbd className="input-shortcut" aria-hidden="true">/</kbd>
    </div>
    <Button className="add-button" type="submit">追加<CornerDownLeft aria-hidden="true" /></Button>
  </form>;
}

function Overview({ total, completed }: { total: number; completed: number }) {
  const active = total - completed;
  const percentage = total ? Math.floor(completed * 100 / total) : 0;
  return <section className="overview" aria-labelledby="page-title">
    <div className="intro">
      <p className="eyebrow"><span className="tiny-square" /> A LITTLE, EVERY DAY.</p>
      <h1 id="page-title">やることを、<br /><span>ひとつずつ。</span></h1>
      <p className="summary">{total === 0 ? '頭の中を軽くして、今日の一歩を。' : active === 0 ? 'すべて完了。おつかれさまでした！' : <>あと <strong>{active}</strong> 件。自分のペースで進めましょう。</>}</p>
    </div>
    <div className="progress-card" role="group" aria-label={`${total}件中${completed}件完了`}>
      <div className="progress-top"><span className="eyebrow">YOUR PROGRESS</span><ArrowUpRight size={18} aria-hidden="true" /></div>
      <div className="progress-number"><strong>{String(percentage).padStart(2, '0')}</strong><span>%</span></div>
      <Progress value={percentage} aria-label="Todoの完了率" className="progress-track" />
      <div className="progress-caption"><span>{completed} / {total} 件完了</span><span>ONE STEP AT A TIME</span></div>
    </div>
  </section>;
}

function FilterNav({ filter, total, active, completed }: { filter: Filter; total: number; active: number; completed: number }) {
  const items = [{ value: 'all', label: 'すべて', count: total }, { value: 'active', label: '未完了', count: active }, { value: 'completed', label: '完了', count: completed }] as const;
  return <nav className="filters" aria-label="Todoの絞り込み">
    {items.map(item => <Button key={item.value} asChild variant={filter === item.value ? 'secondary' : 'ghost'} size="sm">
      <a href={`/?filter=${item.value}`} aria-current={filter === item.value ? 'page' : undefined}>
        {item.label}<span className="filter-count">{item.count}</span>
      </a>
    </Button>)}
  </nav>;
}

function DeleteTodo({ todo, filter }: { todo: Todo; filter: Filter }) {
  return <AlertDialog>
    <AlertDialogTrigger asChild>
      <Button className="delete-button" variant="ghost" size="icon" aria-label={`${todo.title}を削除`}><Trash2 aria-hidden="true" /></Button>
    </AlertDialogTrigger>
    <AlertDialogContent>
      <AlertDialogHeader>
        <AlertDialogTitle>このTodoを削除しますか？</AlertDialogTitle>
        <AlertDialogDescription className="delete-description">「{todo.title}」を削除します。この操作は取り消せません。</AlertDialogDescription>
      </AlertDialogHeader>
      <form action="/todos/delete" method="post">
        <input type="hidden" name="id" value={todo.id} />
        <HiddenFilter filter={filter} />
        <AlertDialogFooter>
          <AlertDialogCancel>キャンセル</AlertDialogCancel>
          <Button type="submit">削除する</Button>
        </AlertDialogFooter>
      </form>
    </AlertDialogContent>
  </AlertDialog>;
}

function TodoRow({ todo, filter }: { todo: Todo; filter: Filter }) {
  return <li className={`todo-row${todo.completed ? ' is-complete' : ''}`} data-todo-id={todo.id}>
    <form className="complete-form" action="/todos/complete" method="post">
      <input type="hidden" name="id" value={todo.id} />
      <input type="hidden" name="completed" value={String(!todo.completed)} />
      <HiddenFilter filter={filter} />
      <Button className="complete-button" variant="ghost" type="submit" aria-pressed={todo.completed} aria-label={`${todo.title}を${todo.completed ? '未完了に戻す' : '完了にする'}`}>
        <span className="todo-check" aria-hidden="true">{todo.completed && <Check size={13} strokeWidth={2.5} />}</span>
        <span className="todo-title">{todo.title}</span>
      </Button>
    </form>
    {todo.completed && <Badge variant="outline" className="done-badge">完了</Badge>}
    <DeleteTodo todo={todo} filter={filter} />
  </li>;
}

function EmptyState({ filter, total }: { filter: Filter; total: number }) {
  const [title, hint] = filter === 'completed'
    ? ['完了したTodoはまだありません', '終わったTodoにチェックを付けると、ここに表示されます。']
    : filter === 'active' && total > 0
      ? ['すべてのTodoが完了しました', 'ひと区切り。少し休憩しましょう。']
      : ['最初のTodoを追加しましょう', '小さなことでも大丈夫。上の欄から追加できます。'];
  return <div className="empty-state">
    <div className="empty-icon" aria-hidden="true">{total > 0 ? <CheckCheck /> : <ListTodo />}</div>
    <h2>{title}</h2><p>{hint}</p>
  </div>;
}

export function App({ todos, filter, error }: InitialState) {
  const completed = todos.filter(todo => todo.completed).length;
  const active = todos.length - completed;
  const visible = todos.filter(todo => filter === 'all' || (filter === 'completed' ? todo.completed : !todo.completed));
  return <>
    <a className="skip-link" href="#main">Todoへ移動</a>
    <div className="app-shell">
      <header className="brand-bar">
        <a className="brand" href="/" aria-label="ひとつずつ。 ホーム"><span className="brand-icon"><Check size={20} strokeWidth={2.5} aria-hidden="true" /></span><span>ひとつずつ。</span></a>
        <Badge variant="outline" className="workspace-badge"><span className="status-dot" />マイリスト</Badge>
      </header>
      <main id="main" tabIndex={-1}>
        <Overview total={todos.length} completed={completed} />
        <section className="todo-panel" aria-label="Todoの管理">
          <div className="section-label"><span className="eyebrow">THE LIST</span><ArrowDown size={15} aria-hidden="true" /></div>
          <AddTodo filter={filter} />
          <p id="input-hint" className="input-hint">思いついたら、まずひとつ。<span><kbd>Enter</kbd> で追加</span></p>
          {error && <Alert className="error-message"><AlertDescription>{error}</AlertDescription></Alert>}
          <div className="list-surface">
            <div className="list-toolbar">
              <FilterNav filter={filter} total={todos.length} active={active} completed={completed} />
              <span className="list-count">{String(visible.length).padStart(2, '0')} ITEMS</span>
            </div>
            <Separator />
            {visible.length ? <ul className="todo-list">{visible.map(todo => <TodoRow key={todo.id} todo={todo} filter={filter} />)}</ul> : <EmptyState filter={filter} total={todos.length} />}
            <Separator />
            <div className="panel-footer"><span><CheckCheck size={14} aria-hidden="true" />チェックして完了</span><span><span className="status-dot" />自動保存</span></div>
          </div>
        </section>
      </main>
      <footer className="app-footer"><span>小さな一歩を、積み重ねよう。</span><span className="footer-mark">HITOTSUZUTSU<span>© 2026</span></span></footer>
    </div>
  </>;
}
