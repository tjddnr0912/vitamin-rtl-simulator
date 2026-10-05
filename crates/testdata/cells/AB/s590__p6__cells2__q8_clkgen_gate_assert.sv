module top;
  logic clk, en; wire gclk;
  initial begin clk = 0; forever #5 clk = ~clk; end
  function logic gate(input logic c, input logic e);
    assert (e !== 1'bx) else $error("gate en x at %0t", $time);
    return c & e;
  endfunction
  assign gclk = gate(clk, en);
  always @(posedge gclk) $display("PG t=%0t", $time);
  initial begin
    en = 1;
    #32 $finish;
  end
  initial #200 $finish;
endmodule
