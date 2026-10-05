`ifdef IV
 `define IN(a,b)  ((a) ==? b)
`else
 `define IN(a,b)  ((a) inside {b})
`endif
module t;
  logic signed [3:0] sa; logic signed [7:0] s8, s8n; logic signed [63:0] s64; logic [3:0] u4; logic signed [4:0] s5;
`ifdef CONSTS
  localparam signed [3:0] SA = -4'sd2; localparam signed [7:0] S8 = 8'sd0, S8N = -8'sd4; localparam signed [63:0] S64 = -64'sd2;
  localparam C41 = `IN(SA >>> 1, 4'sb111?);
  localparam C42 = `IN(S64 >>> 1, 64'shFFFF_FFFF_FFFF_FFF?);
  localparam C43 = `IN(SA + S8, 8'sb1111_111?);
  localparam C44 = `IN(S8N / 8'sd2, 8'sb1111_111?);
  localparam C45 = `IN(SA >>> 1, 8'sb1111_111?);
  localparam C46 = `IN(S8N >>> 1, 4'sb111?);
  localparam C47 = `IN(SA + S8, 4'sb111?);
  localparam C48 = ((SA >>> 1) !=? 4'sb111?);
  localparam C49 = `IN(S8N % 8'sd3, 8'sb1111_111?);
`endif
  initial begin
    sa = -2; s8 = 0; s8n = -4; s64 = -2; u4 = 4'b1110; s5 = -2;
    #1;
    $display("R41 %b", `IN(sa >>> 1, 4'sb111?));
    $display("R42 %b", `IN(s64 >>> 1, 64'shFFFF_FFFF_FFFF_FFF?));
    $display("R43 %b", `IN(sa + s8, 8'sb1111_111?));
    $display("R44 %b", `IN(s8n / 8'sd2, 8'sb1111_111?));
    $display("R45 %b", `IN(sa >>> 1, 8'sb1111_111?));
    $display("R46 %b", `IN(s8n >>> 1, 4'sb111?));
    $display("R47 %b", `IN(sa + s8, 4'sb111?));
    $display("R48 %b", ((sa >>> 1) !=? 4'sb111?));
    $display("R49 %b", `IN(s8n % 8'sd3, 8'sb1111_111?));
    $display("R50 %b", `IN(u4 >>> 1, 4'sb011?));
    $display("R51 %b", `IN(sa >>> 1, 4'b011?));
    $display("R52 %b", `IN(s5 >>> 1, 4'sb111?));
    $display("R53 %b", ((sa >>> 1) == 4'sb1111));
`ifdef CONSTS
    $display("C41 %b", C41); $display("C42 %b", C42); $display("C43 %b", C43); $display("C44 %b", C44); $display("C45 %b", C45);
    $display("C46 %b", C46); $display("C47 %b", C47); $display("C48 %b", C48); $display("C49 %b", C49);
`endif
    #1 $finish;
  end
endmodule
