localparam logic signed [3:0] S4 = -4'sd4;
localparam logic [64:0] W65 = {1'b1, 64'd1};
module m #(parameter int N = 7) (); initial $display("@ovr %0d", N); endmodule
module top;
  localparam L = (16'h00FF ==? 8'sb1???_????);
  typedef enum logic [1:0] {EA = (16'h00FF ==? 8'sb1???_????), EB = (16'h00FF ==? 8'sb1???_????) + 1} e_t;
  localparam logic [7:0] PV = 8'hA5;
  logic arr [(16'h00FF ==? 8'sb1???_????) + 1];
  localparam int LI = (16'h00FF !=? 8'sb1???_????);
  localparam logic [(16'h00FF ==? 8'sb1???_????):0] LB = '1;
  m #(.N((16'h00FF ==? 8'sb1???_????))) u ();
  for (genvar i = 0; i <= (16'h00FF ==? 8'sb1???_????); i++) begin : gf initial $display("@gf %0d", i); end
  initial begin
    $display("@rep %0d", $bits({((16'h00FF ==? 8'sb1???_????) + 1){1'b1}}));
    $display("@cast %0d", $bits(((16'h00FF ==? 8'sb1???_????) + 2)'(4'hF)));
    $display("@enum %0d %0d", EA, EB);
    $display("@psel %0d %b", $bits(PV[(16'h00FF ==? 8'sb1???_????) + 1 : 0]), PV[(16'h00FF ==? 8'sb1???_????) + 1 : 0]);
    $display("@arr %0d", $size(arr));
    $display("@bits %0d", $bits((16'h00FF ==? 8'sb1???_????)));
    $display("@LI %0d LB=%0d L=%b", LI, $bits(LB), L);
  end
endmodule
