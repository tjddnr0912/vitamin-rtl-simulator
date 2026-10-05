import os
quals=["unique","unique0","priority","priority0"]
forms={
 "if":   "    f = 7;\n    {q} if (a == 1) f = 10;\n",
 "case": "    f = 7;\n    {q} case (a) 1: f = 10; 3: f = 30; endcase\n",
 "casez":"    f = 7;\n    {q} casez (a) 1: f = 10; 3: f = 30; endcase\n",
 "casex":"    f = 7;\n    {q} casex (a) 1: f = 10; 3: f = 30; endcase\n",
 "inside":"    f = 7;\n    {q} case (a) inside 1: f = 10; [3:4]: f = 30; endcase\n",
}
for q in quals:
  for fn,body in forms.items():
    for reach,arg in (("M",2),("N",1)):
      name=f"a_{q}_{fn}_{reach}"
      src=("module top;\n  function automatic int f(input int a);\n"+body.format(q=q)+
           "  endfunction\n  localparam int P = f(%d);\n  initial begin #1 $display(\"P=%%0d\", P); $finish; end\nendmodule\n")%arg
      open(name+".sv","w").write(src)
