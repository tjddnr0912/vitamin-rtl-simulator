module top;
  localparam logic [1:0] MODE = 2'd2;
  wire [1:0] mode_w = MODE;
  logic clk = 0;
  function automatic logic en_of(input logic [1:0] m);
    assert (m != 2'd3) else $error("bad mode");
    en_of = (m == 2'd2);
  endfunction
  wire en = en_of(mode_w);
  logic [3:0] cnt = 0;
  always @(posedge clk) if (en) cnt <= cnt + 1;
  initial begin
    if (en !== 1'b1) $display("t0 en=%b NOT READY", en); else $display("t0 en ready");
  end
  always #5 clk = ~clk;
  initial #22 begin $display("cnt=%0d", cnt); $finish; end
endmodule
