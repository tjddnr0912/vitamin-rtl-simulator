module top;
  logic a = 0, b = 0;
  logic [1:0] r = 0, y;
  function automatic logic [1:0] fchain(input logic x, input logic z);
    logic [1:0] v;
    v = 0;
    if (x) v = 1; else unique if (z) v = 2;
    return v;
  endfunction
  function automatic logic [1:0] fsingle(input logic x);
    logic [1:0] v;
    v = 0;
    unique if (x) v = 1;
    return v;
  endfunction
  task automatic tchain(input logic x, input logic z, output logic [1:0] o);
    o = 0;
    if (x) o = 1; else unique if (z) o = 2;
  endtask
  task automatic tsingle(input logic x, output logic [1:0] o);
    o = 0;
    unique if (x) o = 1;
  endtask
  initial begin
    #1 y = fchain(a, b);  $display("t=%0t fchain done", $time);
    #1 y = fsingle(a);    $display("t=%0t fsingle done", $time);
    #1 tchain(a, b, y);   $display("t=%0t tchain done", $time);
    #1 tsingle(a, y);     $display("t=%0t tsingle done", $time);
    #1 $finish;
  end
  initial #100 $finish;
endmodule
