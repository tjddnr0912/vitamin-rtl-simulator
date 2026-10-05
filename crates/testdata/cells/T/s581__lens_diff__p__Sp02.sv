localparam logic signed [3:0] S4 = -4'sd4;
localparam logic [64:0] W65 = {1'b1, 64'd1};
module m #(parameter int N = 7) (); initial $display("@ovr %0d", N); endmodule
module top;
  localparam L = (33'h1_0000_0000 ==? 33'h1_????_???0);
  typedef enum logic [1:0] {EA = (33'h1_0000_0000 ==? 33'h1_????_???0), EB = (33'h1_0000_0000 ==? 33'h1_????_???0) + 1} e_t;
  localparam logic [7:0] PV = 8'hA5;
  logic arr [(33'h1_0000_0000 ==? 33'h1_????_???0) + 1];
  localparam int LI = (33'h1_0000_0000 !=? 33'h1_????_???0);
  localparam logic [(33'h1_0000_0000 ==? 33'h1_????_???0):0] LB = '1;
  m #(.N((33'h1_0000_0000 ==? 33'h1_????_???0))) u ();
  for (genvar i = 0; i <= (33'h1_0000_0000 ==? 33'h1_????_???0); i++) begin : gf initial $display("@gf %0d", i); end
  initial begin
    $display("@rep %0d", $bits({((33'h1_0000_0000 ==? 33'h1_????_???0) + 1){1'b1}}));
    $display("@cast %0d", $bits(((33'h1_0000_0000 ==? 33'h1_????_???0) + 2)'(4'hF)));
    $display("@enum %0d %0d", EA, EB);
    $display("@psel %0d %b", $bits(PV[(33'h1_0000_0000 ==? 33'h1_????_???0) + 1 : 0]), PV[(33'h1_0000_0000 ==? 33'h1_????_???0) + 1 : 0]);
    $display("@arr %0d", $size(arr));
    $display("@bits %0d", $bits((33'h1_0000_0000 ==? 33'h1_????_???0)));
    $display("@LI %0d LB=%0d L=%b", LI, $bits(LB), L);
  end
endmodule
