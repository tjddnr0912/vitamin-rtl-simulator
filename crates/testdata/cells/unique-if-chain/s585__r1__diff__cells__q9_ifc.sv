interface ifc;
  logic s;
  task it(input logic x, input logic z);
    unique if (x) $display("it x");
    else if (z) $display("it z");
  endtask
  function void ifv(input logic x, input logic z);
    unique if (x) $display("ifv x");
    else if (z) $display("ifv z");
  endfunction
endinterface
program prg(input logic a, input logic b);
  task pt(input logic x, input logic z);
    unique if (x) $display("pt x");
    else if (z) $display("pt z");
  endtask
  initial begin
    #3 pt(a, b);
  end
endprogram
module top;
  logic a = 0, b = 0;
  ifc u();
  prg pp(.a(a), .b(b));
  function static void fs(input logic x, input logic z);
    unique if (x) $display("fs x");
    else if (z) $display("fs z");
  endfunction
  if (1) begin : g
    function automatic void gv(input logic x, input logic z);
      unique if (x) $display("gv x");
      else if (z) $display("gv z");
    endfunction
    initial #4 gv(a, b);
  end
  initial begin
    #1 u.it(a, b);
    #1 u.ifv(a, b);
    #3 fs(a, b);
    #1 $display("t=%0t end", $time);
    #1 $finish;
  end
  initial #100 $finish;
endmodule
