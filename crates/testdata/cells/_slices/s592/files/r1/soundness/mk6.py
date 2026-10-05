import os
D = os.path.dirname(os.path.abspath(__file__)) + '/cells'
T = 'initial #5 $finish;'
def lp(step_inner):
    return f"""module top;
  localparam S = 1;
  if (1) begin : b
    for (genvar i = 0; i < 3; i = i + S) begin : g
      wire [3:0] w = i;
      initial #1 $display("@%m w=%0d", w);
    end
    {step_inner}
  end
  {T}
endmodule
"""
cells = {'STALL1_step_zero': lp('localparam S = 0;'), 'NEG1_step_neg': lp('localparam S = -1;')}
for k, v in cells.items():
    open(f'{D}/{k}.sv', 'w').write(v)
