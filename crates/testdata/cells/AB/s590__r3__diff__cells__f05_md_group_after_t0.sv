module top;
  logic e1, e2; logic [3:0] a;
  function automatic logic [3:0] f(input logic [3:0] x, input integer id);
    $display("f%0d t=%0t x=%0d", id, $time, x);
    f = x + 4'd1;
  endfunction
  tri [3:0] w;
  assign w = e1 ? f(a, 1) : 4'bz;
  assign w = e2 ? (a ^ 4'hF) : 4'bz;
  always @(w) $display("ev w=%b t=%0t", w, $time);
  initial begin
    a = 2; e1 = 1; e2 = 0;
    #2 e1 = 0; e2 = 1;
    #2 a = 5;
    #2 e1 = 1; e2 = 0;
    #2 $display("t8 w=%b", w);
    $finish;
  end
  initial #50 $finish;
endmodule
