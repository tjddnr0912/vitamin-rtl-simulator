localparam logic signed [3:0] S4 = -4'sd4;
localparam logic [64:0] W65 = {1'b1, 64'd1};
module m #(parameter int N = 7) (); initial $display("@ovr %0d", N); endmodule
module top;
  localparam L = (64'hFFFF_FFFF_FFFF_FFFE ==? 'bx1);
  typedef enum logic [1:0] {EA = (64'hFFFF_FFFF_FFFF_FFFE ==? 'bx1), EB = (64'hFFFF_FFFF_FFFF_FFFE ==? 'bx1) + 1} e_t;
  localparam logic [7:0] PV = 8'hA5;
  logic arr [(64'hFFFF_FFFF_FFFF_FFFE ==? 'bx1) + 1];
  localparam int LI = (64'hFFFF_FFFF_FFFF_FFFE !=? 'bx1);
  localparam logic [(64'hFFFF_FFFF_FFFF_FFFE ==? 'bx1):0] LB = '1;
  m #(.N((64'hFFFF_FFFF_FFFF_FFFE ==? 'bx1))) u ();
  for (genvar i = 0; i <= (64'hFFFF_FFFF_FFFF_FFFE ==? 'bx1); i++) begin : gf initial $display("@gf %0d", i); end
  initial begin
    $display("@rep %0d", $bits({((64'hFFFF_FFFF_FFFF_FFFE ==? 'bx1) + 1){1'b1}}));
    $display("@cast %0d", $bits(((64'hFFFF_FFFF_FFFF_FFFE ==? 'bx1) + 2)'(4'hF)));
    $display("@enum %0d %0d", EA, EB);
    $display("@psel %0d %b", $bits(PV[(64'hFFFF_FFFF_FFFF_FFFE ==? 'bx1) + 1 : 0]), PV[(64'hFFFF_FFFF_FFFF_FFFE ==? 'bx1) + 1 : 0]);
    $display("@arr %0d", $size(arr));
    $display("@bits %0d", $bits((64'hFFFF_FFFF_FFFF_FFFE ==? 'bx1)));
    $display("@LI %0d LB=%0d L=%b", LI, $bits(LB), L);
  end
endmodule
