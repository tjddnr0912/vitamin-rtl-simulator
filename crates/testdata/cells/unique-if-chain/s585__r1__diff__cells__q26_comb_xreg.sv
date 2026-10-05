module top;
  logic clk = 0, rst_n = 0;
  logic [1:0] sel;
  logic [3:0] y;
  always #5 clk = ~clk;
  always_ff @(posedge clk or negedge rst_n)
    if (!rst_n) sel <= 2'd0;
    else sel <= sel + 2'd1;
  always_comb begin
    y = 4'd0;
    unique if (sel == 2'd0) y = 4'd1;
    else if (sel == 2'd1) y = 4'd2;
    else if (sel == 2'd2) y = 4'd4;
    else if (sel == 2'd3) y = 4'd8;
  end
  initial begin
    #2 rst_n = 1;
    #40 $display("t=%0t y=%0d", $time, y);
    $finish;
  end
endmodule
