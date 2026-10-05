module top;
  logic clk, en; wire gclk; int n;
  initial begin clk = 0; forever #5 clk = ~clk; end
  function logic gate(input logic c, input logic e);
    assert (e !== 1'bx) else $error("gate en x at %0t", $time);
    return c & e;
  endfunction
  assign gclk = gate(clk, en);
  always @(posedge gclk) n++;
  always @(negedge gclk) $display("NG t=%0t", $time);
  initial begin
    en = 1;
    #15; @(posedge clk) $display("S t=%0t n=%0d", $time, n);
    #30 $display("t=%0t n=%0d", $time, n);
    $finish;
  end
  initial #200 $finish;
endmodule
