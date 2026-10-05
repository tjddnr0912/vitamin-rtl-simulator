module top;
  logic [1:0] r = 2'b01;
  logic [1:0] y1, y2;
  function automatic logic [1:0] f(input logic [1:0] v);
    unique casez (v)
      2'b?1: f = 1;
      2'b1?: f = 2;
    endcase
  endfunction
  initial begin
    #5 $display("P1 call t=%0t", $time);
    y1 = f(2'b00);
  end
  always_comb begin
    $display("P2 eval t=%0t r=%b", $time, r);
    y2 = f(r);
  end
  initial begin
    #5 r = 2'b00;
    #0 r = 2'b01;
    #1 $display("t=%0t done", $time);
    #1 $finish;
  end
  initial #100 $finish;
endmodule
