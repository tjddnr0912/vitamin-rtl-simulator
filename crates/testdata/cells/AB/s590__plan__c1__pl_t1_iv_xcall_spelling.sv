module top;
  logic a; logic [1:0] av;
  logic y1, y2, y3;
  function logic f1(input logic x);
    $display("f1 t=%0t x=%b", $time, x);
    unique case (x) 1'b0: f1 = 1; 1'b1: f1 = 0; endcase
  endfunction
  function logic f2(input logic x);
    $display("f2 t=%0t x=%b", $time, x);
    unique case (x) 1'b0: f2 = 1; 1'b1: f2 = 0; endcase
  endfunction
  function logic f3(input logic x);
    $display("f3 t=%0t x=%b", $time, x);
    unique case (x) 1'b0: f3 = 1; 1'b1: f3 = 0; endcase
  endfunction
  assign y1 = f1(a);
  assign y2 = f2(a | 1'b0);
  assign y3 = f3(av[0]);
  initial begin
    a = 0; av = 2'b10;
    #1 $display("t=%0t y1=%b y2=%b y3=%b", $time, y1, y2, y3);
    #1 $finish;
  end
  initial #100 $finish;
endmodule
