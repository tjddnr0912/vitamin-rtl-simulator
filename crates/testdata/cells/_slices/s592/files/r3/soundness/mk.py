import os
D = os.path.dirname(os.path.abspath(__file__)) + '/cells'
def case(a, b, tail, la='g', lb='g', pre=''):
    return f"""module top;
{pre}  localparam K = 4;
  if (1) begin : gb
    case (8'd4)
      K: begin : {la} {a} end
      default: begin : {lb} {b} end
    endcase
    localparam K = 8;
  end
{tail}
  initial #5 $finish;
endmodule
"""
cells = {
 'SN1_hierparam_samelabel': case('localparam P = 1;', 'localparam P = 2;', '  initial #1 $display("@P=%0d", gb.g.P);'),
 'SN2_hierparam_otherarm': case('localparam P = 1;', 'localparam P = 2;', '  initial #1 $display("@P=%0d", gb.g2.P);', 'g1', 'g2'),
 'SN3_hierparam_recarm': case('localparam P = 1;', 'localparam P = 2;', '  initial #1 $display("@P=%0d", gb.g1.P);', 'g1', 'g2'),
 'AS1_assert_in_arm': case('ap: assert property (@(posedge clk) 1\'b0) else $display("@fail %m");', '', '', pre='  reg clk = 0;\n  always #1 clk = ~clk;\n'),
 'CS1_str_init_arm': case('string s = "a";', 'string s = "b";', '  initial #1 $display("@s=%s", gb.g.s);'),
 'CM1_multiname_init': case("logic [3:0] a, v = 4'd9;", "logic [7:0] a, v = 8'd200;", '  initial #1 $display("@v=%0d bits=%0d", gb.g.v, $bits(gb.g.v));'),
 'SN4_for_hierparam': """module top;
  localparam N = 1;
  if (1) begin : gb
    for (genvar i = 0; i < N; i++) begin : L localparam P = i + 10; end
    localparam N = 3;
  end
  initial #1 $display("@P=%0d", gb.L[2].P);
  initial #5 $finish;
endmodule
""",
}
for k, v in cells.items():
    open(f'{D}/{k}.sv', 'w').write(v)
print(len(cells))
